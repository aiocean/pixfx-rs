"""PPO for the duel's policy (CleanRL-style, one file): many envs, short rollouts, masked actions, an HL-Gauss critic.

    uv run python ppo.py --run v1 [--updates 3000] [--envs 512] [--device cpu|mps] [--resume runs/v1/last.pt]
    uv run python ppo.py --run v2 --init runs/v1/last.pt --updates 600 --lr 1e-4 --ent 0.003 0.002 --shaping-until 0

`--resume` continues a run where it stopped (its optimiser and its place in the schedule); `--init` starts a new run,
with its own schedule, from another run's network (a `.pt` checkpoint, critic included, or a `.bin` weights file).

Writes runs/<run>/log.jsonl (one line every 10 updates) and runs/<run>/last.pt (every 25 updates and at the end).
"""

import argparse
import json
import pathlib
import time
from collections import defaultdict, deque

import numpy as np
import torch
from torch.distributions import Categorical

import pixfx_duel as pd
from league import League
from net import Policy, hl_gauss, load_bin


def schedule(update, updates, lr, ent, shaping_until):
    """Learning rate, entropy weight and shaping at `update` of `updates`: the first two fall linearly to 0 and to
    `ent[1]`, shaping falls from 1 to 0 over the first `shaping_until` of the run (0: no shaping at all)."""
    frac = update / updates
    shaping = max(0.0, 1 - frac / shaping_until) if shaping_until > 0 else 0.0
    return lr * (1 - frac), ent[0] + (ent[1] - ent[0]) * frac, shaping


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", required=True)
    ap.add_argument("--updates", type=int, default=3000)
    ap.add_argument("--envs", type=int, default=512)
    ap.add_argument("--steps", type=int, default=64)
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--gamma", type=float, default=0.995)
    ap.add_argument("--lam", type=float, default=0.95)
    ap.add_argument("--clip", type=float, default=0.2)
    ap.add_argument("--epochs", type=int, default=4)
    ap.add_argument("--minibatches", type=int, default=8)
    ap.add_argument("--ent", type=float, nargs=2, default=[0.01, 0.002])
    ap.add_argument("--shaping-until", type=float, default=0.6, help="share of updates over which shaping falls to 0")
    ap.add_argument("--snapshot-every", type=int, default=20)
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--thunder", type=float, default=0.5, help="share of fights in which the learner plays Thunder")
    ap.add_argument("--resume")
    ap.add_argument("--init", help="start from this network (.pt or .bin) with a fresh schedule")
    args = ap.parse_args()

    out = pathlib.Path(__file__).parent / "runs" / args.run
    out.mkdir(parents=True, exist_ok=True)
    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    dev = torch.device(args.device)
    n, T = args.envs, args.steps

    policy = Policy(pd.OBS, pd.ACTIONS).to(dev)
    opt = torch.optim.Adam(policy.parameters(), lr=args.lr, eps=1e-5)
    start = 0
    env = pd.VecEnv(n, args.threads)
    league = League(env, rng, seed=1_000_000 + args.seed * 10_000_000, thunder=args.thunder)
    if args.init:
        path = pathlib.Path(args.init)
        if path.suffix == ".pt":
            policy.load_state_dict(torch.load(path, map_location=dev, weights_only=False)["model"])
        else:
            load_bin(policy, path.read_bytes())
    if args.resume:
        ck = torch.load(args.resume, map_location=dev, weights_only=False)
        policy.load_state_dict(ck["model"])
        opt.load_state_dict(ck["optim"])
        start = ck["update"]
        league.next_seed = ck["next_seed"]
    league.snapshot(policy.cpu())
    policy.to(dev)
    kinds = [league.reset(i) for i in range(n)]

    obs_b = torch.zeros(T, n, pd.OBS)
    mask_b = torch.zeros(T, n, pd.ACTIONS, dtype=torch.bool)
    act_b = torch.zeros(T, n, dtype=torch.long)
    logp_b = torch.zeros(T, n)
    val_b = torch.zeros(T, n)
    rew_b = torch.zeros(T, n)
    done_b = torch.zeros(T, n)
    results = defaultdict(lambda: deque(maxlen=400))  # opponent -> recent outcomes (1 won, 0 lost or cut)
    lengths = deque(maxlen=400)
    acts = np.zeros(pd.ACTIONS)
    log = open(out / "log.jsonl", "a")
    tick = time.time()

    for update in range(start, args.updates):
        lr, ent_coef, shaping = schedule(update, args.updates, args.lr, args.ent, args.shaping_until)
        for g in opt.param_groups:
            g["lr"] = lr

        policy.eval()
        for t in range(T):
            obs, mask = env.observe()
            o, m = torch.from_numpy(obs), torch.from_numpy(mask)
            with torch.no_grad():
                logits, vlog = policy(o.to(dev), m.to(dev))
                dist = Categorical(logits=logits)
                a = dist.sample()
                obs_b[t], mask_b[t], act_b[t] = o, m, a.cpu()
                logp_b[t] = dist.log_prob(a).cpu()
                val_b[t] = policy.value_of(vlog).cpu()
            r, d, won, length = env.step(a.cpu().numpy().astype(np.int64), shaping)
            np.add.at(acts, a.cpu().numpy(), 1)
            rew_b[t], done_b[t] = torch.from_numpy(r), torch.from_numpy(d.astype(np.float32))
            for i in np.nonzero(d)[0]:
                kind, style, school = kinds[i]
                results[kind].append(float(won[i] == 1))
                results[f"{kind}/{('thunder', 'frost')[school]}"].append(float(won[i] == 1))
                if kind == "brain":
                    results[f"brain/style{style}"].append(float(won[i] == 1))
                lengths.append(float(length[i]))
                kinds[i] = league.reset(int(i))

        with torch.no_grad():
            obs, mask = env.observe()
            _, vlog = policy(torch.from_numpy(obs).to(dev), torch.from_numpy(mask).to(dev))
            nxt = policy.value_of(vlog).cpu()
        adv = torch.zeros(T, n)
        gae = torch.zeros(n)
        for t in reversed(range(T)):
            live = 1.0 - done_b[t]
            v_next = nxt if t == T - 1 else val_b[t + 1]
            delta = rew_b[t] + args.gamma * v_next * live - val_b[t]
            gae = delta + args.gamma * args.lam * live * gae
            adv[t] = gae
        ret = adv + val_b

        policy.train()
        B = T * n
        flat = lambda x: x.reshape(B, *x.shape[2:]).to(dev)
        O, M, A, LP, ADV, RET = map(flat, (obs_b, mask_b, act_b, logp_b, adv, ret))
        mb = B // args.minibatches
        stats = defaultdict(float)
        for _ in range(args.epochs):
            perm = torch.randperm(B, device=dev)
            for k in range(args.minibatches):
                idx = perm[k * mb : (k + 1) * mb]
                logits, vlog = policy(O[idx], M[idx])
                dist = Categorical(logits=logits)
                ratio = (dist.log_prob(A[idx]) - LP[idx]).exp()
                a_n = (ADV[idx] - ADV[idx].mean()) / (ADV[idx].std() + 1e-8)
                pg = torch.max(-a_n * ratio, -a_n * ratio.clamp(1 - args.clip, 1 + args.clip)).mean()
                vloss = -(hl_gauss(RET[idx], policy.support, policy.sigma) * vlog.log_softmax(-1)).sum(-1).mean()
                ent = dist.entropy().mean()
                loss = pg + vloss - ent_coef * ent
                opt.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(policy.parameters(), 0.5)
                opt.step()
                stats["pg"] += pg.item()
                stats["v"] += vloss.item()
                stats["ent"] += ent.item()
                stats["clipfrac"] += ((ratio - 1).abs() > args.clip).float().mean().item()

        if (update + 1) % args.snapshot_every == 0:
            league.snapshot(policy.cpu())
            policy.to(dev)
        if (update + 1) % 10 == 0:
            k = args.epochs * args.minibatches
            line = {
                "update": update + 1,
                "steps": (update + 1) * B,
                "sps": round(10 * B / (time.time() - tick)),
                "shaping": round(shaping, 3),
                **{f: round(v / k, 4) for f, v in stats.items()},
                "len": round(float(np.mean(lengths)), 1) if lengths else 0,
                "top_act": round(float(acts.max() / max(acts.sum(), 1)), 3),
                **{f"win/{o}": round(float(np.mean(v)), 3) for o, v in sorted(results.items())},
            }
            acts[:] = 0
            tick = time.time()
            print(json.dumps(line), flush=True)
            log.write(json.dumps(line) + "\n")
            log.flush()
        if (update + 1) % 25 == 0 or update + 1 == args.updates:
            torch.save(
                {"model": policy.state_dict(), "optim": opt.state_dict(), "update": update + 1, "next_seed": league.next_seed},
                out / "last.pt",
            )


if __name__ == "__main__":
    main()
