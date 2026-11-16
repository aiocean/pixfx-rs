import numpy as np

from league import League


class Recorder:
    """Stands in for the VecEnv: keeps the learner corner (0 Thunder, 1 Frost) of every reset."""

    def __init__(self):
        self.corners = []

    def set_weights(self, wid, data):
        pass

    def reset(self, i, seed, learner, style, foe_style, name, wid, limit):
        self.corners.append(learner)


def thunder_share(fights=4000, **kw):
    """The share of `fights` fresh fights in which the learner sits in Thunder's corner."""
    env = Recorder()
    league = League(env, np.random.default_rng(7), **kw)
    league.snapshots = [0]
    for i in range(fights):
        league.reset(i)
    return env.corners.count(0) / fights


def test_the_learner_plays_thunder_in_the_share_it_is_given():
    assert abs(thunder_share(thunder=0.75) - 0.75) < 0.03


def test_the_learner_plays_each_school_half_the_time_by_default():
    assert abs(thunder_share() - 0.5) < 0.03
