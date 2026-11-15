"""Write a policy as the weights file pixfx-duel ships, and the check its Rust test reads: 32 observations from
real fights, their masks, and PyTorch's logits for them.

    uv run python export.py runs/<run>/last.pt            # a checkpoint
    uv run python export.py ../crates/pixfx-duel/weights/policy.bin   # re-check a weights file as it is
"""

import argparse
import json
import pathlib

import numpy as np
import torch

import pixfx_duel as pd
from net import Policy, load_bin, to_bin

HERE = pathlib.Path(__file__).parent
WEIGHTS = HERE / "../crates/pixfx-duel/weights/policy.bin"
CHECK = HERE / "../crates/pixfx-duel/tests/fixtures/policy_check.json"


def observations(n=32, seed=424242):
    """n observations, with their masks, from fights a few seconds in against each opponent."""
    env = pd.VecEnv(n, 4)
    opponents = ["brain", "first", "aggressive", "turtle"]
    for i in range(n):
        env.reset(i, seed + i, i % 2, i % 3, (i + 2) % 3, opponents[i % 4], -1, 90.0)
    rng = np.random.default_rng(seed)
    for _ in range(int(rng.integers(20, 40))):
        obs, mask = env.observe()
        actions = np.where(mask, rng.random(mask.shape), -1).argmax(1).astype(np.int64)
        env.step(actions, 0.0)
    return env.observe()


def load(path):
    policy = Policy(pd.OBS, pd.ACTIONS)
    path = pathlib.Path(path)
    if path.suffix == ".pt":
        policy.load_state_dict(torch.load(path, map_location="cpu", weights_only=False)["model"])
    else:
        load_bin(policy, path.read_bytes())
    return policy.eval()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("source")
    ap.add_argument("--out", default=str(WEIGHTS))
    ap.add_argument("--check", default=str(CHECK))
    args = ap.parse_args()
    policy = load(args.source)
    data = to_bin(policy)
    pathlib.Path(args.out).write_bytes(data)
    obs, mask = observations()
    with torch.no_grad():
        logits, _ = policy(torch.from_numpy(obs), torch.ones(mask.shape, dtype=torch.bool))
    cases = [
        {"obs": [float(x) for x in o], "mask": [bool(m) for m in k], "logits": [float(x) for x in l]}
        for o, k, l in zip(obs, mask, logits.numpy())
    ]
    pathlib.Path(args.check).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(args.check).write_text(json.dumps({"source": pathlib.Path(args.source).name, "cases": cases}))
    print(f"wrote {args.out} ({len(data)} bytes) and {args.check} ({len(cases)} cases)")


if __name__ == "__main__":
    main()
