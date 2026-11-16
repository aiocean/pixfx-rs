//! The policy network, run in plain `f32` in one fixed order on every target (no FMA, each row summed in 8 partial
//! sums joined in a fixed tree; `games/arena/train/net.py` mirrors it), so every target computes the same bits: `relu(in · x)`,
//! two residual blocks `h + down(relu(up(ln(h))))`, a final LayerNorm, a simplicial embedding (`sem`: 16 softmaxes
//! of 8), and the policy head (`pi`). The trainer's value head is not shipped.
//!
//! The weights file: magic `PXNET1\0\0`, a little-endian `u32` tensor count, then per tensor its `u32` name length,
//! UTF-8 name, `u32` rank, `u32` dims and `f32` data, row-major.

use super::{ACTIONS, OBS};
use crate::rng::Rng;
use core::fmt;

/// The width of the trunk.
pub const WIDTH: usize = 128;
/// The width inside a residual block.
pub const HIDDEN: usize = 256;
/// The size of each simplex of the simplicial embedding.
pub const GROUP: usize = 8;
const BLOCKS: usize = 2;
/// The partial sums of a row.
const LANES: usize = 8;
const MAGIC: &[u8; 8] = b"PXNET1\0\0";
const EPS: f32 = 1e-5;

/// A tensor as stored: its name, its dims, its data.
pub type Tensor = (String, Vec<u32>, Vec<f32>);

/// Why a weights file was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum NetError {
    /// Not a network file (or one from another layout).
    Magic,
    /// The file ends inside `what`.
    Truncated { what: String },
    /// A tensor the network needs is not there.
    Missing { name: String },
    /// A tensor has the wrong dims.
    Shape {
        name: String,
        want: Vec<u32>,
        got: Vec<u32>,
    },
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::Magic => write!(f, "not a policy network file (magic {MAGIC:?} missing)"),
            NetError::Truncated { what } => write!(f, "policy network file ends inside {what}"),
            NetError::Missing { name } => write!(f, "policy network file has no tensor {name}"),
            NetError::Shape { name, want, got } => {
                write!(f, "policy network tensor {name} is {got:?}, want {want:?}")
            }
        }
    }
}

impl std::error::Error for NetError {}

/// A dense layer: `out x inp` weights, row-major, and `out` biases.
#[derive(Clone, Debug, PartialEq)]
struct Dense {
    w: Vec<f32>,
    b: Vec<f32>,
    inp: usize,
}

impl Dense {
    /// `out = w · x + b`. Each row is summed in `LANES` partial sums, lane `k` taking every `LANES`-th product from
    /// `k`, then joined in a fixed tree: one order on every target, and no chain of dependent adds.
    fn apply(&self, x: &[f32], out: &mut [f32]) {
        for (o, y) in out.iter_mut().enumerate() {
            let row = self.w.get(o * self.inp..(o + 1) * self.inp).unwrap_or(&[]);
            let mut acc = [0.0_f32; LANES];
            let (rows, rest) = row.as_chunks::<LANES>();
            let (xs, xrest) = x.get(..row.len()).unwrap_or(&[]).as_chunks::<LANES>();
            for (w, v) in rows.iter().zip(xs) {
                for k in 0..LANES {
                    acc[k] += w[k] * v[k];
                }
            }
            for (k, (w, v)) in rest.iter().zip(xrest).enumerate() {
                acc[k] += w * v;
            }
            let s =
                ((acc[0] + acc[1]) + (acc[2] + acc[3])) + ((acc[4] + acc[5]) + (acc[6] + acc[7]));
            *y = s + self.b.get(o).copied().unwrap_or(0.0);
        }
    }
}

/// A LayerNorm's gain and bias.
#[derive(Clone, Debug, PartialEq)]
struct Norm {
    g: Vec<f32>,
    b: Vec<f32>,
}

impl Norm {
    fn apply(&self, x: &[f32], out: &mut [f32]) {
        let n = x.len() as f32;
        let mut mean = 0.0;
        for v in x {
            mean += v;
        }
        mean /= n;
        let mut var = 0.0;
        for v in x {
            let d = v - mean;
            var += d * d;
        }
        var /= n;
        let inv = 1.0 / libm::sqrtf(var + EPS);
        for (i, y) in out.iter_mut().enumerate() {
            let v = x.get(i).copied().unwrap_or(0.0);
            *y = (v - mean) * inv * self.g.get(i).copied().unwrap_or(1.0)
                + self.b.get(i).copied().unwrap_or(0.0);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Block {
    ln: Norm,
    up: Dense,
    down: Dense,
}

/// The policy network.
#[derive(Clone, Debug, PartialEq)]
pub struct Net {
    input: Dense,
    blocks: Vec<Block>,
    out: Norm,
    sem: Dense,
    pi: Dense,
}

/// Every tensor's name and dims, in file order.
fn layout() -> Vec<(String, Vec<u32>)> {
    let (w, h, o, a) = (WIDTH as u32, HIDDEN as u32, OBS as u32, ACTIONS as u32);
    let mut l = vec![
        ("in.w".to_string(), vec![w, o]),
        ("in.b".to_string(), vec![w]),
    ];
    for k in 0..BLOCKS {
        l.push((format!("block{k}.ln.g"), vec![w]));
        l.push((format!("block{k}.ln.b"), vec![w]));
        l.push((format!("block{k}.up.w"), vec![h, w]));
        l.push((format!("block{k}.up.b"), vec![h]));
        l.push((format!("block{k}.down.w"), vec![w, h]));
        l.push((format!("block{k}.down.b"), vec![w]));
    }
    l.extend([
        ("out.ln.g".to_string(), vec![w]),
        ("out.ln.b".to_string(), vec![w]),
        ("sem.w".to_string(), vec![w, w]),
        ("sem.b".to_string(), vec![w]),
        ("pi.w".to_string(), vec![a, w]),
        ("pi.b".to_string(), vec![a]),
    ]);
    l
}

/// Reads little-endian values off the front of a byte slice.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize, what: &str) -> Result<&[u8], NetError> {
        if self.0.len() < n {
            return Err(NetError::Truncated {
                what: what.to_string(),
            });
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn u32(&mut self, what: &str) -> Result<u32, NetError> {
        let b = self.take(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

impl Net {
    /// Read a weights file, refusing anything that is not exactly this network.
    pub fn load(bytes: &[u8]) -> Result<Net, NetError> {
        let mut r = Reader(bytes);
        if r.take(MAGIC.len(), "the magic")
            .map_err(|_| NetError::Magic)?
            != MAGIC
        {
            return Err(NetError::Magic);
        }
        let count = r.u32("the tensor count")?;
        let mut found: Vec<Tensor> = Vec::new();
        for _ in 0..count {
            let len = r.u32("a tensor name")? as usize;
            let name = String::from_utf8_lossy(r.take(len, "a tensor name")?).into_owned();
            let rank = r.u32(&name)? as usize;
            let mut dims = Vec::with_capacity(rank);
            for _ in 0..rank {
                dims.push(r.u32(&name)?);
            }
            let n: usize = dims.iter().map(|&d| d as usize).product();
            let raw = r.take(n.saturating_mul(4), &name)?;
            let data = raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            found.push((name, dims, data));
        }
        let mut take = |name: &str, want: &[u32]| -> Result<Vec<f32>, NetError> {
            let i = found
                .iter()
                .position(|(n, _, _)| n == name)
                .ok_or(NetError::Missing {
                    name: name.to_string(),
                })?;
            let (_, dims, data) = found.swap_remove(i);
            if dims != want {
                return Err(NetError::Shape {
                    name: name.to_string(),
                    want: want.to_vec(),
                    got: dims,
                });
            }
            Ok(data)
        };
        let mut tensors = Vec::new();
        for (name, dims) in layout() {
            let data = take(&name, &dims)?;
            tensors.push((name, dims, data));
        }
        Ok(Net::from_tensors(tensors))
    }

    /// The network from tensors already checked against `layout`, in its order.
    fn from_tensors(tensors: Vec<Tensor>) -> Net {
        let mut it = tensors.into_iter().map(|(_, _, d)| d);
        let mut next = || it.next().unwrap_or_default();
        let input = Dense {
            w: next(),
            b: next(),
            inp: OBS,
        };
        let blocks = (0..BLOCKS)
            .map(|_| Block {
                ln: Norm {
                    g: next(),
                    b: next(),
                },
                up: Dense {
                    w: next(),
                    b: next(),
                    inp: WIDTH,
                },
                down: Dense {
                    w: next(),
                    b: next(),
                    inp: HIDDEN,
                },
            })
            .collect();
        let out = Norm {
            g: next(),
            b: next(),
        };
        let sem = Dense {
            w: next(),
            b: next(),
            inp: WIDTH,
        };
        let pi = Dense {
            w: next(),
            b: next(),
            inp: WIDTH,
        };
        Net {
            input,
            blocks,
            out,
            sem,
            pi,
        }
    }

    /// A network of seeded random weights (uniform within `1/sqrt(fan in)`, norms at 1 and 0): a stand-in until a
    /// trained one exists, and a fixed network for tests.
    pub fn seeded(seed: u32) -> Net {
        let mut rng = Rng::new(seed);
        let tensors = layout()
            .into_iter()
            .map(|(name, dims)| {
                let n: usize = dims.iter().map(|&d| d as usize).product();
                let data = if name.ends_with(".g") {
                    vec![1.0; n]
                } else if name.ends_with(".ln.b") {
                    vec![0.0; n]
                } else {
                    let fan = dims.last().copied().unwrap_or(1).max(1) as f64;
                    let s = 1.0 / libm::sqrt(fan);
                    (0..n)
                        .map(|_| ((rng.next() * 2.0 - 1.0) * s) as f32)
                        .collect()
                };
                (name, dims, data)
            })
            .collect();
        Net::from_tensors(tensors)
    }

    /// Every tensor, in file order.
    pub fn tensors(&self) -> Vec<Tensor> {
        let mut data = vec![self.input.w.clone(), self.input.b.clone()];
        for b in &self.blocks {
            data.extend([
                b.ln.g.clone(),
                b.ln.b.clone(),
                b.up.w.clone(),
                b.up.b.clone(),
                b.down.w.clone(),
                b.down.b.clone(),
            ]);
        }
        data.extend([
            self.out.g.clone(),
            self.out.b.clone(),
            self.sem.w.clone(),
            self.sem.b.clone(),
            self.pi.w.clone(),
            self.pi.b.clone(),
        ]);
        layout()
            .into_iter()
            .zip(data)
            .map(|((n, d), v)| (n, d, v))
            .collect()
    }

    /// The network as a weights file.
    pub fn to_bytes(&self) -> Vec<u8> {
        Net::write(&self.tensors())
    }

    /// Tensors as a weights file.
    pub fn write(tensors: &[Tensor]) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend((tensors.len() as u32).to_le_bytes());
        for (name, dims, data) in tensors {
            out.extend((name.len() as u32).to_le_bytes());
            out.extend(name.as_bytes());
            out.extend((dims.len() as u32).to_le_bytes());
            for d in dims {
                out.extend(d.to_le_bytes());
            }
            for v in data {
                out.extend(v.to_le_bytes());
            }
        }
        out
    }

    /// The policy's logits for an observation.
    pub fn logits(&self, obs: &[f32; OBS]) -> [f32; ACTIONS] {
        let mut h = [0.0_f32; WIDTH];
        self.input.apply(obs, &mut h);
        relu(&mut h);
        let mut n = [0.0_f32; WIDTH];
        let mut up = [0.0_f32; HIDDEN];
        let mut down = [0.0_f32; WIDTH];
        for b in &self.blocks {
            b.ln.apply(&h, &mut n);
            b.up.apply(&n, &mut up);
            relu(&mut up);
            b.down.apply(&up, &mut down);
            for (x, d) in h.iter_mut().zip(&down) {
                *x += d;
            }
        }
        self.out.apply(&h, &mut n);
        let mut s = [0.0_f32; WIDTH];
        self.sem.apply(&n, &mut s);
        for g in s.chunks_mut(GROUP) {
            softmax(g);
        }
        let mut logits = [0.0_f32; ACTIONS];
        self.pi.apply(&s, &mut logits);
        logits
    }
}

fn relu(x: &mut [f32]) {
    for v in x {
        *v = v.max(0.0);
    }
}

fn softmax(x: &mut [f32]) {
    let m = x.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for v in x.iter_mut() {
        *v = libm::expf(*v - m);
        sum += *v;
    }
    for v in x.iter_mut() {
        *v /= sum;
    }
}

/// Pick an action from `logits` with a draw `u` in [0, 1): a softmax over the legal actions only, read by its
/// inverse CDF. Always a legal action, and `Wait` (0) when nothing is legal.
pub fn sample(logits: &[f32; ACTIONS], legal: &[bool; ACTIONS], u: f32) -> usize {
    let mut m = f32::NEG_INFINITY;
    let mut last = None;
    for (i, (&l, &ok)) in logits.iter().zip(legal).enumerate() {
        if ok {
            m = m.max(l);
            last = Some(i);
        }
    }
    let Some(last) = last else { return 0 };
    let mut p = [0.0_f32; ACTIONS];
    let mut sum = 0.0;
    for ((pi, &l), &ok) in p.iter_mut().zip(logits).zip(legal) {
        if ok {
            *pi = libm::expf(l - m);
            sum += *pi;
        }
    }
    let target = u * sum;
    let mut acc = 0.0;
    for (i, (&pi, &ok)) in p.iter().zip(legal).enumerate() {
        if ok {
            acc += pi;
            if acc > target {
                return i;
            }
        }
    }
    last
}
