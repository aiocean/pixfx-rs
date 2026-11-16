"""The policy network, mirroring pixfx-mind's (`crates/pixfx-mind/src/policy/net.rs`) layer for layer, plus the
value head the trainer alone uses: a critic trained by HL-Gauss classification over fixed bins.

The weights file is the one `Net::load` reads: magic b"PXNET1\\0\\0", a little-endian u32 tensor count, then per
tensor its u32 name length, name, u32 rank, u32 dims and float32 data, row-major. `torch.nn.Linear.weight` is
[out, in], the order Rust reads.
"""

import math
import struct

import torch
from torch import nn

MAGIC = b"PXNET1\0\0"
WIDTH, HIDDEN, GROUP, BLOCKS = 128, 256, 8, 2


class Block(nn.Module):
    """x + down(relu(up(ln(x))))"""

    def __init__(self, width, hidden):
        super().__init__()
        self.ln = nn.LayerNorm(width, eps=1e-5)
        self.up = nn.Linear(width, hidden)
        self.down = nn.Linear(hidden, width)

    def forward(self, x):
        return x + self.down(torch.relu(self.up(self.ln(x))))


class Policy(nn.Module):
    """relu(in), two residual blocks, a LayerNorm, a simplicial embedding (16 softmaxes of 8), the policy head; and
    the value head (bins over [-1.5, 1.5]) from the trunk's normalised output."""

    def __init__(self, obs, actions, width=WIDTH, hidden=HIDDEN, bins=101, vmax=1.5):
        super().__init__()
        self.inp = nn.Linear(obs, width)
        self.blocks = nn.ModuleList([Block(width, hidden) for _ in range(BLOCKS)])
        self.out_ln = nn.LayerNorm(width, eps=1e-5)
        self.sem = nn.Linear(width, width)
        self.pi = nn.Linear(width, actions)
        self.value = nn.Sequential(nn.Linear(width, width), nn.ReLU(), nn.Linear(width, bins))
        self.bins = bins
        self.register_buffer("support", torch.linspace(-vmax, vmax, bins + 1))
        self.sigma = 0.75 * (2 * vmax / bins)
        self.width = width

    def forward(self, obs, mask):
        """Masked logits (an illegal act gets -1e8) and the value logits."""
        h = torch.relu(self.inp(obs))
        for b in self.blocks:
            h = b(h)
        z = self.out_ln(h)
        s = self.sem(z).view(*z.shape[:-1], self.width // GROUP, GROUP).softmax(-1).flatten(-2)
        logits = self.pi(s).masked_fill(~mask, -1e8)
        return logits, self.value(z)

    def value_of(self, value_logits):
        """The critic's expected return."""
        centres = (self.support[:-1] + self.support[1:]) / 2
        return (value_logits.softmax(-1) * centres).sum(-1)

    def tensors(self):
        """Every shipped tensor in the order and with the names Rust reads."""
        out = [("in.w", self.inp.weight), ("in.b", self.inp.bias)]
        for k, b in enumerate(self.blocks):
            out += [
                (f"block{k}.ln.g", b.ln.weight),
                (f"block{k}.ln.b", b.ln.bias),
                (f"block{k}.up.w", b.up.weight),
                (f"block{k}.up.b", b.up.bias),
                (f"block{k}.down.w", b.down.weight),
                (f"block{k}.down.b", b.down.bias),
            ]
        out += [
            ("out.ln.g", self.out_ln.weight),
            ("out.ln.b", self.out_ln.bias),
            ("sem.w", self.sem.weight),
            ("sem.b", self.sem.bias),
            ("pi.w", self.pi.weight),
            ("pi.b", self.pi.bias),
        ]
        return out


def hl_gauss(returns, support, sigma):
    """HL-Gauss targets: a Gaussian of width sigma round each return, integrated over each bin."""
    cdf = torch.special.erf((support[None, :] - returns[:, None].clamp(support[0], support[-1])) / (math.sqrt(2) * sigma))
    p = cdf[:, 1:] - cdf[:, :-1]
    return p / p.sum(-1, keepdim=True).clamp_min(1e-12)


def to_bin(policy):
    """The policy as a weights file."""
    tensors = policy.tensors()
    out = bytearray(MAGIC + struct.pack("<I", len(tensors)))
    for name, t in tensors:
        data = t.detach().to("cpu", torch.float32).contiguous()
        out += struct.pack("<I", len(name)) + name.encode()
        out += struct.pack("<I", data.dim()) + b"".join(struct.pack("<I", d) for d in data.shape)
        out += data.numpy().astype("<f4").tobytes()
    return bytes(out)


def load_bin(policy, data):
    """Load a weights file's tensors into `policy` (its value head is left as it is)."""
    if data[:8] != MAGIC:
        raise ValueError("not a policy network file")
    (count,), at = struct.unpack_from("<I", data, 8), 12
    found = {}
    for _ in range(count):
        (n,) = struct.unpack_from("<I", data, at)
        name = data[at + 4 : at + 4 + n].decode()
        at += 4 + n
        (rank,) = struct.unpack_from("<I", data, at)
        dims = struct.unpack_from(f"<{rank}I", data, at + 4)
        at += 4 + 4 * rank
        size = math.prod(dims)
        found[name] = torch.frombuffer(bytearray(data[at : at + 4 * size]), dtype=torch.float32).reshape(dims)
        at += 4 * size
    with torch.no_grad():
        for name, t in policy.tensors():
            if name not in found:
                raise ValueError(f"no tensor {name}")
            if tuple(found[name].shape) != tuple(t.shape):
                raise ValueError(f"tensor {name} is {tuple(found[name].shape)}, want {tuple(t.shape)}")
            t.copy_(found[name])
