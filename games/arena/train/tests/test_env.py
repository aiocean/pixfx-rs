"""The batched duel environments: random legal play ends every episode with a round's outcome, and replays."""

import numpy as np
import pixfx_duel as pd


def random_episodes(n, seed, steps=3000):
    env = pd.VecEnv(n, 4)
    for i in range(n):
        env.reset(i, seed + i, i % 2, i % 3, (i + 1) % 3, ["brain", "first", "aggressive", "turtle"][i % 4], -1, 90.0)
    rng = np.random.default_rng(seed)
    finals = np.zeros(n, dtype=np.float32)
    ended = np.zeros(n, dtype=bool)
    rewards = []
    for _ in range(steps):
        obs, mask = env.observe()
        assert obs.shape == (n, pd.OBS) and obs.dtype == np.float32
        assert mask.shape == (n, pd.ACTIONS) and mask.dtype == bool
        assert mask[:, 0].all(), "waiting is always legal"
        logits = rng.random((n, pd.ACTIONS))
        actions = np.where(mask, logits, -1.0).argmax(axis=1).astype(np.int64)
        reward, done, won, length = env.step(actions, 0.0)
        rewards.append(reward.copy())
        newly = done & ~ended
        finals[newly] = reward[newly]
        ended |= done
        if ended.all():
            break
    return ended, finals, np.array(rewards)


def test_random_legal_play_finishes_episodes():
    ended, finals, _ = random_episodes(16, 1)
    assert ended.all()
    assert np.all(np.abs(np.abs(finals) - 1.0) < 0.05), finals


def test_same_seeds_same_rewards():
    a = random_episodes(8, 7)[2]
    b = random_episodes(8, 7)[2]
    assert np.array_equal(a, b)


def test_a_policy_opponent_from_weights():
    env = pd.VecEnv(2, 1)
    env.set_weights(3, open("../crates/pixfx-duel/weights/policy.bin", "rb").read())
    env.reset(0, 1, 0, 1, 1, "policy", 3, 90.0)
    env.reset(1, 2, 1, 1, 0, "policy", 3, 90.0)
    obs, _ = env.observe()
    assert np.isfinite(obs).all()
    for _ in range(3000):
        _, done, _, _ = env.step(np.zeros(2, dtype=np.int64), 0.0)
        if done.all():
            break
    assert done.all()


def test_unknown_opponent_or_weights_is_an_error():
    env = pd.VecEnv(1, 1)
    for args in [("nobody", -1), ("policy", 99)]:
        try:
            env.reset(0, 1, 0, 1, 1, args[0], args[1], 90.0)
        except ValueError:
            continue
        raise AssertionError(f"{args} accepted")
