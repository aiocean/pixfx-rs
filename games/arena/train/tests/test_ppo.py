"""The trainer's schedule: learning rate and entropy fall over the run, shaping fades to 0 (or is off)."""

from ppo import schedule


def test_schedule_starts_full_and_ends_at_its_floors():
    lr, ent, shaping = schedule(0, 100, lr=3e-4, ent=(0.01, 0.002), shaping_until=0.6)
    assert (lr, ent, shaping) == (3e-4, 0.01, 1.0)
    lr, ent, shaping = schedule(100, 100, lr=3e-4, ent=(0.01, 0.002), shaping_until=0.6)
    assert lr == 0.0 and abs(ent - 0.002) < 1e-12 and shaping == 0.0


def test_shaping_until_zero_turns_shaping_off():
    for u in (0, 50, 99):
        assert schedule(u, 100, lr=1e-4, ent=(0.003, 0.002), shaping_until=0.0)[2] == 0.0
