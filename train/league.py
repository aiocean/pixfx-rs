"""Who the learner fights: its latest self, its past selves, the brain, the first mind and the scripted fighters.
Snapshots are handed to the envs as weights ids (`VecEnv.set_weights`)."""

import numpy as np

from net import to_bin

# opponent shares: latest self, a past self, the brain, the first mind, the scripted pair
MIX = {"latest": 0.35, "past": 0.25, "brain": 0.15, "first": 0.10, "scripted": 0.15}
STYLES = 3


class League:
    def __init__(self, env, rng, limit=90.0, seed=1_000_000):
        self.env, self.rng, self.limit = env, rng, limit
        self.snapshots = []  # weights ids, oldest first
        self.next_seed = seed  # training fights never use the ladder's seeds (1-150)
        self.kinds = list(MIX)
        self.p = np.array([MIX[k] for k in self.kinds])

    def snapshot(self, policy):
        """Freeze the policy as it is now: the latest self from here on."""
        wid = len(self.snapshots)
        self.env.set_weights(wid, to_bin(policy))
        self.snapshots.append(wid)

    def reset(self, i):
        """Start env i on a fresh fight against an opponent drawn from the mix; returns the opponent's name."""
        kind = self.kinds[self.rng.choice(len(self.kinds), p=self.p)]
        wid = -1
        if kind == "latest" or (kind == "past" and len(self.snapshots) < 2):
            name, wid = "policy", self.snapshots[-1]
            kind = "latest"
        elif kind == "past":
            name, wid = "policy", self.snapshots[int(self.rng.integers(0, len(self.snapshots) - 1))]
        elif kind == "scripted":
            name = ["aggressive", "turtle"][int(self.rng.integers(0, 2))]
            kind = name
        else:
            name = kind
        seed = self.next_seed
        self.next_seed += 1
        learner = int(self.rng.integers(0, 2))
        style, foe_style = (int(x) for x in self.rng.integers(0, STYLES, 2))
        self.env.reset(i, seed, learner, style, foe_style, name, wid, self.limit)
        return kind, style
