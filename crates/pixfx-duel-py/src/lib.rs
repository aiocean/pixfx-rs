//! The landing duel's training environments for Python (`import pixfx_duel`): `VecEnv` runs many
//! `pixfx_duel::train::Env`s, each built and stepped on one worker thread (an env holds `Rc`s and seats, so it
//! never moves between threads); only plain arrays cross. It translates only: rewards, episodes and opponents are
//! `pixfx_duel::train`'s.

use numpy::{PyArray1, PyArrayMethods, PyReadonlyArray1};
use pixfx_duel::style::Style;
use pixfx_duel::train::{Env, EnvConfig, Opponent};
use pixfx_mind::policy::{ACTIONS, Net, OBS};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

/// How a reset names its opponent.
#[derive(Clone, Copy, Debug)]
enum Foe {
    Brain,
    First,
    Aggressive,
    Turtle,
    Policy(u32),
}

/// One env's latest decision, and what its last step earned.
#[derive(Clone, Debug)]
struct Out {
    obs: Vec<f32>,
    mask: Vec<bool>,
    reward: f32,
    done: bool,
    won: i8,
    length: f32,
}

impl Out {
    fn blank() -> Out {
        Out { obs: vec![0.0; OBS], mask: vec![false; ACTIONS], reward: 0.0, done: true, won: 0, length: 0.0 }
    }
}

/// What the main thread asks a worker.
enum Wire {
    Weights { id: u32, bytes: Vec<u8> },
    Reset { slot: usize, seed: u32, learner: usize, style: Style, foe_style: Style, foe: Foe, limit: f64, reply: Sender<Result<Out, String>> },
    Step { actions: Vec<i64>, shaping: f32, reply: Sender<Vec<Out>> },
}

/// A worker's envs and the networks it has been handed.
struct Worker {
    envs: Vec<Option<Env>>,
    nets: HashMap<u32, Rc<Net>>,
    effects: Vec<String>,
}

impl Worker {
    fn run(n: usize, rx: Receiver<Wire>) {
        let effects = pixfx_effects::EFFECTS.iter().map(|e| e.meta().id.to_string()).collect();
        let mut w = Worker { envs: (0..n).map(|_| None).collect(), nets: HashMap::new(), effects };
        while let Ok(msg) = rx.recv() {
            w.answer(msg);
        }
    }

    fn answer(&mut self, msg: Wire) {
        match msg {
            Wire::Weights { id, bytes } => {
                if let Ok(net) = Net::load(&bytes) {
                    self.nets.insert(id, Rc::new(net));
                }
            }
            Wire::Reset { slot, seed, learner, style, foe_style, foe, limit, reply } => {
                let _ = reply.send(self.reset(slot, seed, learner, style, foe_style, foe, limit));
            }
            Wire::Step { actions, shaping, reply } => {
                let outs = self
                    .envs
                    .iter_mut()
                    .zip(actions)
                    .map(|(env, a)| match env {
                        Some(env) => {
                            let s = env.step(usize::try_from(a).unwrap_or(0), shaping);
                            let mut out = snapshot(env);
                            out.reward = s.reward;
                            out.done = s.done;
                            out.won = match s.won {
                                Some(true) => 1,
                                Some(false) => -1,
                                None => 0,
                            };
                            out.length = s.length as f32;
                            out
                        }
                        None => Out::blank(),
                    })
                    .collect();
                let _ = reply.send(outs);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reset(&mut self, slot: usize, seed: u32, learner: usize, style: Style, foe_style: Style, foe: Foe, limit: f64) -> Result<Out, String> {
        let opponent = match foe {
            Foe::Brain => Opponent::Brain,
            Foe::First => Opponent::First,
            Foe::Aggressive => Opponent::Aggressive,
            Foe::Turtle => Opponent::Turtle,
            Foe::Policy(id) => Opponent::Policy(self.nets.get(&id).cloned().ok_or(format!("no weights {id}: set_weights first"))?),
        };
        let cfg = EnvConfig { seed, learner, style, foe_style, opponent, limit, effects: self.effects.clone() };
        let env = Env::new(cfg);
        let out = snapshot(&env);
        let place = self.envs.get_mut(slot).ok_or(format!("no env slot {slot}"))?;
        *place = Some(env);
        Ok(Out { done: false, ..out })
    }
}

/// An env's current decision.
fn snapshot(env: &Env) -> Out {
    let (obs, mask) = env.observe();
    Out { obs: obs.to_vec(), mask: mask.to_vec(), reward: 0.0, done: false, won: 0, length: 0.0 }
}

/// Many duel environments stepped together.
#[pyclass(unsendable)]
struct VecEnv {
    n: usize,
    /// Each worker's channel and the first env index it owns.
    workers: Vec<(Sender<Wire>, usize, usize)>,
    handles: Vec<JoinHandle<()>>,
    outs: Vec<Out>,
}

impl VecEnv {
    /// The worker that owns env `i`, and `i`'s slot in it.
    fn owner(&self, i: usize) -> Option<(&Sender<Wire>, usize)> {
        self.workers.iter().find(|(_, start, len)| i >= *start && i < start + len).map(|(tx, start, _)| (tx, i - start))
    }
}

fn style(i: usize) -> PyResult<Style> {
    Style::ALL.get(i).copied().ok_or_else(|| PyValueError::new_err(format!("style {i}: 0 aggressive, 1 balanced, 2 defensive")))
}

#[pymethods]
impl VecEnv {
    /// `n` envs on `threads` worker threads. Each must be `reset` before it steps.
    #[new]
    fn new(n: usize, threads: usize) -> PyResult<VecEnv> {
        let threads = threads.clamp(1, n.max(1));
        let mut workers = Vec::new();
        let mut handles = Vec::new();
        let mut start = 0;
        for t in 0..threads {
            let len = n / threads + usize::from(t < n % threads);
            let (tx, rx) = channel();
            let handle = std::thread::Builder::new()
                .name(format!("duel-env-{t}"))
                .spawn(move || Worker::run(len, rx))
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            workers.push((tx, start, len));
            handles.push(handle);
            start += len;
        }
        Ok(VecEnv { n, workers, handles, outs: (0..n).map(|_| Out::blank()).collect() })
    }

    /// Hand every worker a network as `id`, for `policy` opponents.
    fn set_weights(&self, id: u32, data: Vec<u8>) -> PyResult<()> {
        Net::load(&data).map_err(|e| PyValueError::new_err(e.to_string()))?;
        for (tx, _, _) in &self.workers {
            tx.send(Wire::Weights { id, bytes: data.clone() }).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Start env `i` on a fresh round: the fight's `seed`, the learner's corner (0 Thunder, 1 Frost) and style
    /// (0 aggressive, 1 balanced, 2 defensive), the opponent's style, the opponent (`brain`, `first`, `aggressive`,
    /// `turtle`, `policy`), the weights id of a `policy` opponent, and the seconds a round may last.
    #[allow(clippy::too_many_arguments)]
    fn reset(&mut self, i: usize, seed: u32, learner: usize, style_: usize, foe_style: usize, opponent: &str, weights: i64, limit: f64) -> PyResult<()> {
        let foe = match opponent {
            "brain" => Foe::Brain,
            "first" => Foe::First,
            "aggressive" => Foe::Aggressive,
            "turtle" => Foe::Turtle,
            "policy" => Foe::Policy(u32::try_from(weights).map_err(|_| PyValueError::new_err("a policy opponent needs its weights id"))?),
            other => return Err(PyValueError::new_err(format!("no opponent {other:?}"))),
        };
        if learner > 1 {
            return Err(PyValueError::new_err(format!("learner corner {learner}: 0 or 1")));
        }
        let (style, foe_style) = (style(style_)?, style(foe_style)?);
        let (tx, slot) = self.owner(i).ok_or_else(|| PyValueError::new_err(format!("no env {i} of {}", self.n)))?;
        let (reply, rx) = channel();
        tx.send(Wire::Reset { slot, seed, learner, style, foe_style, foe, limit, reply }).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let out = rx.recv().map_err(|e| PyRuntimeError::new_err(e.to_string()))?.map_err(PyValueError::new_err)?;
        if let Some(o) = self.outs.get_mut(i) {
            *o = out;
        }
        Ok(())
    }

    /// Every env's observation (`float32[n, OBS]`) and legal actions (`bool[n, ACTIONS]`).
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
        let obs: Vec<f32> = self.outs.iter().flat_map(|o| o.obs.iter().copied()).collect();
        let mask: Vec<bool> = self.outs.iter().flat_map(|o| o.mask.iter().copied()).collect();
        let obs = PyArray1::from_vec(py, obs).reshape([self.n, OBS])?;
        let mask = PyArray1::from_vec(py, mask).reshape([self.n, ACTIONS])?;
        Ok((obs.into_any(), mask.into_any()))
    }

    /// Step every env with `actions` (`int64[n]`; an env that is done or never reset ignores its action):
    /// `(reward float32[n], done bool[n], won int8[n] (1 won, -1 lost, 0 neither), length float32[n])`.
    #[allow(clippy::type_complexity)]
    fn step<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<'py, i64>,
        shaping: f32,
    ) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>, Bound<'py, PyAny>, Bound<'py, PyAny>)> {
        let actions = actions.as_slice()?.to_vec();
        if actions.len() != self.n {
            return Err(PyValueError::new_err(format!("{} actions for {} envs", actions.len(), self.n)));
        }
        let workers: Vec<(Sender<Wire>, usize, usize)> = self.workers.clone();
        let done_before: Vec<bool> = self.outs.iter().map(|o| o.done).collect();
        let outs = py.detach(move || -> Result<Vec<Vec<Out>>, String> {
            let mut replies = Vec::new();
            for (tx, start, len) in &workers {
                let (reply, rx) = channel();
                let slice = actions.get(*start..start + len).unwrap_or(&[]).to_vec();
                tx.send(Wire::Step { actions: slice, shaping, reply }).map_err(|e| e.to_string())?;
                replies.push(rx);
            }
            replies.into_iter().map(|rx| rx.recv().map_err(|e| e.to_string())).collect()
        });
        let outs = outs.map_err(PyRuntimeError::new_err)?;
        for (i, out) in outs.into_iter().flatten().enumerate() {
            let Some(o) = self.outs.get_mut(i) else { continue };
            if done_before.get(i).copied().unwrap_or(true) {
                *o = Out { reward: 0.0, done: true, won: 0, ..o.clone() };
            } else {
                *o = out;
            }
        }
        let reward: Vec<f32> = self.outs.iter().map(|o| o.reward).collect();
        let done: Vec<bool> = self.outs.iter().map(|o| o.done).collect();
        let won: Vec<i8> = self.outs.iter().map(|o| o.won).collect();
        let length: Vec<f32> = self.outs.iter().map(|o| o.length).collect();
        Ok((
            PyArray1::from_vec(py, reward).into_any(),
            PyArray1::from_vec(py, done).into_any(),
            PyArray1::from_vec(py, won).into_any(),
            PyArray1::from_vec(py, length).into_any(),
        ))
    }
}

impl Drop for VecEnv {
    fn drop(&mut self) {
        self.workers.clear();
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

#[pymodule]
#[pyo3(name = "pixfx_duel")]
fn duel_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VecEnv>()?;
    m.add("OBS", OBS)?;
    m.add("ACTIONS", ACTIONS)?;
    Ok(())
}
