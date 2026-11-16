"""The trainer's network mirrors pixfx-mind's, reads and writes its weights file, and trains its critic by HL-Gauss."""

import numpy as np
import torch

import pixfx_duel as pd
from net import Policy, hl_gauss, load_bin, to_bin


def test_forward_shapes_and_mask():
    p = Policy(pd.OBS, pd.ACTIONS)
    obs = torch.randn(5, pd.OBS)
    mask = torch.zeros(5, pd.ACTIONS, dtype=torch.bool)
    mask[:, 0] = True
    mask[2, 7] = True
    logits, value = p(obs, mask)
    assert logits.shape == (5, pd.ACTIONS) and value.shape == (5, p.bins)
    probs = torch.softmax(logits, -1)
    assert torch.all(probs[~mask] < 1e-12)
    assert torch.allclose(probs.sum(-1), torch.ones(5))


def test_hl_gauss_targets_sum_to_one_and_centre_on_the_return():
    p = Policy(pd.OBS, pd.ACTIONS)
    target = hl_gauss(torch.tensor([-1.0, 0.0, 0.37, 1.4]), p.support, p.sigma)
    assert torch.allclose(target.sum(-1), torch.ones(4), atol=1e-5)
    centres = (p.support[:-1] + p.support[1:]) / 2
    assert torch.allclose((target * centres).sum(-1), torch.tensor([-1.0, 0.0, 0.37, 1.4]), atol=0.02)


def test_a_weights_file_round_trips(tmp_path):
    p = Policy(pd.OBS, pd.ACTIONS)
    path = tmp_path / "p.bin"
    path.write_bytes(to_bin(p))
    q = Policy(pd.OBS, pd.ACTIONS)
    load_bin(q, path.read_bytes())
    obs = torch.randn(3, pd.OBS)
    mask = torch.ones(3, pd.ACTIONS, dtype=torch.bool)
    assert torch.equal(p(obs, mask)[0], q(obs, mask)[0])


def test_the_rust_stand_in_loads():
    q = Policy(pd.OBS, pd.ACTIONS)
    load_bin(q, open("../crates/pixfx-duel/weights/policy.bin", "rb").read())
    logits, _ = q(torch.zeros(1, pd.OBS), torch.ones(1, pd.ACTIONS, dtype=torch.bool))
    assert np.isfinite(logits.detach().numpy()).all()
