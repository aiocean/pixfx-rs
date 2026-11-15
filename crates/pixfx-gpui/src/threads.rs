//! Painting on native threads: each effect's state lives on one thread for its life, which steps it, draws the
//! frame, crops it and turns it into gpui images; the window's thread only takes the images back
//! ([`ThreadPainter::take`]). A frame asked for shows from the window frame after it comes back; until then the
//! effect shows the frame it had.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use pixfx_core::{Actor, Clip, Layer, Placement, Sim};
use pixfx_draw::Frame;
use pixfx_scene::{EffectKey, Job, Painter, Target};

use crate::Painted;

/// One layer to paint, sent to the thread that keeps the effect's state.
struct Paint {
    effect: EffectKey,
    /// The clip, sent with the effect's first job only: the thread keeps it with the state.
    clip: Option<Clip>,
    frame: u64,
    fade: f64,
    place: Placement,
    layer: Layer,
    target: Target,
    /// The character the effect plays on, handed to its state before it is brought to `frame`.
    actor: Option<Arc<Actor>>,
}

enum ToThread {
    Paint(Box<Paint>),
    /// The effect is gone: its state goes. Sent after its last job, on the same channel.
    End(EffectKey),
}

/// A layer painted on a thread, as images ready for the window.
pub(crate) struct Done {
    pub(crate) effect: EffectKey,
    pub(crate) layer: Layer,
    pub(crate) target: Target,
    pub(crate) painted: Painted,
}

/// Paints on `n` native threads. Dropping it ends the threads once they finished the jobs they have.
pub struct ThreadPainter {
    threads: Vec<(Sender<ToThread>, JoinHandle<()>)>,
    back: Receiver<Done>,
    /// The thread each effect's state lives on.
    owner: HashMap<EffectKey, usize>,
    /// The thread the next effect goes to.
    next: usize,
}

impl ThreadPainter {
    /// `n` threads, at least one.
    pub fn new(n: usize) -> std::io::Result<Self> {
        let (done, back) = channel();
        let threads = (0..n.max(1))
            .map(|i| {
                let (tx, rx) = channel();
                let done = done.clone();
                let handle = std::thread::Builder::new()
                    .name(format!("pixfx-paint-{i}"))
                    .spawn(move || run(&rx, &done))?;
                Ok((tx, handle))
            })
            .collect::<std::io::Result<Vec<_>>>()?;
        Ok(Self {
            threads,
            back,
            owner: HashMap::new(),
            next: 0,
        })
    }

    /// The layers painted since the last call, in the order they came back.
    pub(crate) fn take(&mut self) -> Vec<Done> {
        self.back.try_iter().collect()
    }
}

impl Painter for ThreadPainter {
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame> {
        let n = self.threads.len();
        let mut first = false;
        let owner = *self.owner.entry(job.effect).or_insert_with(|| {
            first = true;
            let t = self.next;
            self.next = (self.next + 1) % n;
            t
        });
        let paint = Paint {
            effect: job.effect,
            clip: first.then(|| job.clip.clone()),
            frame: job.frame,
            fade: job.fade,
            place: job.place,
            layer: job.layer,
            target: job.target.clone(),
            actor: job.actor.map(|a| Arc::clone(&a.actor)),
        };
        // a thread that is gone (it panicked) paints nothing: the effect keeps the frame it had
        if let Some((tx, _)) = self.threads.get(owner) {
            let _ = tx.send(ToThread::Paint(Box::new(paint)));
        }
        None
    }

    fn end(&mut self, effect: EffectKey) {
        if let Some(owner) = self.owner.remove(&effect)
            && let Some((tx, _)) = self.threads.get(owner)
        {
            let _ = tx.send(ToThread::End(effect));
        }
    }
}

impl Drop for ThreadPainter {
    fn drop(&mut self) {
        for (tx, handle) in self.threads.drain(..) {
            // closing the channel ends the thread's loop
            drop(tx);
            let _ = handle.join();
        }
    }
}

/// A thread's loop: keep each effect's clip, state and frame, paint each job, send the images back.
fn run(rx: &Receiver<ToThread>, done: &Sender<Done>) {
    let mut sims: HashMap<EffectKey, (Clip, Sim, Frame)> = HashMap::new();
    for msg in rx {
        match msg {
            ToThread::End(effect) => {
                sims.remove(&effect);
            }
            ToThread::Paint(p) => {
                let p = *p;
                if let Some(clip) = p.clip {
                    let (sim, frame) = (clip.sim(), clip.blank());
                    sims.insert(p.effect, (clip, sim, frame));
                }
                let Some((clip, sim, frame)) = sims.get_mut(&p.effect) else {
                    continue;
                };
                sim.set_actor(p.actor);
                sim.seek(p.frame);
                clip.draw(sim, frame, p.layer, p.place, p.fade);
                let painted = Painted::of(frame);
                if done
                    .send(Done {
                        effect: p.effect,
                        layer: p.layer,
                        target: p.target,
                        painted,
                    })
                    .is_err()
                {
                    // the painter is gone
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use pixfx_core::{Actor, Clip, Layer, Options};
    use pixfx_scene::{ActorFrame, EffectKey, InlinePainter, Job, OwnAt, Painter, Target};

    use super::{Done, ThreadPainter};
    use crate::Painted;

    /// The images' sizes and bytes, colour then light.
    fn pixels(p: &Painted) -> Vec<(u32, u32, u32, u32, Vec<u8>)> {
        [&p.color, &p.light]
            .into_iter()
            .flatten()
            .map(|s| {
                let bytes = s.image.as_bytes(0).map(<[u8]>::to_vec).unwrap_or_default();
                (s.rect.x, s.rect.y, s.rect.w, s.rect.h, bytes)
            })
            .collect()
    }

    /// Wait up to a second for `n` layers to come back.
    fn wait(p: &mut ThreadPainter, n: usize) -> Vec<Done> {
        let mut got = Vec::new();
        let until = Instant::now() + Duration::from_secs(1);
        while got.len() < n && Instant::now() < until {
            got.extend(p.take());
            std::thread::sleep(Duration::from_millis(1));
        }
        got
    }

    #[test]
    fn layers_painted_on_threads_are_the_images_painted_on_the_spot()
    -> Result<(), Box<dyn std::error::Error>> {
        let fx = pixfx_effects::effect("fire/campfire").ok_or("no campfire")?;
        let clips: Vec<Clip> = (0..3)
            .map(|seed| {
                Clip::live(
                    fx,
                    Options {
                        seed: Some(seed),
                        ..Options::default()
                    },
                )
            })
            .collect();
        let mut threads = ThreadPainter::new(2)?;
        let mut inline = InlinePainter::default();
        let mut expected = Vec::new();
        // three effects over two threads, each brought to frames out of order and back
        for frame in [3, 9, 4] {
            for (i, clip) in clips.iter().enumerate() {
                let job = Job {
                    effect: EffectKey {
                        scene: 0,
                        handle: i as u32 + 1,
                    },
                    clip,
                    frame,
                    fade: 1.0,
                    place: clip.placement(),
                    layer: Layer::Both,
                    target: Target::Own(OwnAt {
                        clock: frame,
                        place: clip.placement(),
                        actor: 0,
                    }),
                    actor: None,
                };
                assert!(threads.paint(&job).is_none(), "painted elsewhere");
                let painted = inline.paint(&job).ok_or("inline paints")?;
                expected.push((job.effect, frame, pixels(&Painted::of(painted))));
            }
        }
        let done = wait(&mut threads, expected.len());
        let mut got: Vec<_> = done
            .iter()
            .map(|d| match d.target {
                Target::Own(at) => (d.effect, at.clock, pixels(&d.painted)),
                Target::Shared(_) => (d.effect, u64::MAX, Vec::new()),
            })
            .collect();
        let key = |x: &(EffectKey, u64, Vec<_>)| (x.0, x.1);
        got.sort_by_key(key);
        expected.sort_by_key(key);
        assert_eq!(got.len(), 9);
        assert!(
            expected.iter().all(|e| !e.2.is_empty()),
            "every layer draws something"
        );
        assert!(got == expected, "same images, byte for byte");
        Ok(())
    }

    #[test]
    fn an_actor_painted_on_threads_gives_the_images_painted_on_the_spot()
    -> Result<(), Box<dyn std::error::Error>> {
        let fx = pixfx_effects::effect("feedback/power-up").ok_or("no power-up")?;
        let clip = Clip::live(fx, Options::default());
        let effect = EffectKey {
            scene: 0,
            handle: 1,
        };
        // a 6-wide sprite standing on the origin, then a 2-wide one
        let sprite = |w: u32, version| -> Result<ActorFrame, pixfx_core::ActorError> {
            let actor = Actor::new(
                -(w as i32) / 2,
                -20,
                w,
                20,
                vec![0xff80_c0e0; (w * 20) as usize],
            )?;
            Ok(ActorFrame {
                version,
                actor: std::sync::Arc::new(actor),
            })
        };
        let actors = [sprite(6, 1)?, sprite(2, 2)?];
        let mut threads = ThreadPainter::new(1)?;
        let mut inline = InlinePainter::default();
        let mut expected = Vec::new();
        for (frame, actor) in [(30, &actors[0]), (40, &actors[1]), (50, &actors[0])] {
            let job = Job {
                effect,
                clip: &clip,
                frame,
                fade: 1.0,
                place: clip.placement(),
                layer: Layer::Back,
                target: Target::Own(OwnAt {
                    clock: frame,
                    place: clip.placement(),
                    actor: actor.version,
                }),
                actor: Some(actor),
            };
            threads.paint(&job);
            let painted = inline.paint(&job).ok_or("inline paints")?;
            expected.push(pixels(&Painted::of(painted)));
        }
        let got: Vec<_> = wait(&mut threads, 3)
            .iter()
            .map(|d| pixels(&d.painted))
            .collect();
        assert_eq!(got.len(), 3);
        assert_ne!(expected[0], expected[1], "the aura follows the sprite");
        assert!(got == expected, "same images, byte for byte");
        Ok(())
    }

    #[test]
    fn an_effect_ended_and_played_again_under_its_key_still_paints()
    -> Result<(), Box<dyn std::error::Error>> {
        let fx = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
        let clip = Clip::live(fx, Options::default());
        let mut threads = ThreadPainter::new(1)?;
        let effect = EffectKey {
            scene: 0,
            handle: 1,
        };
        let job = |frame| Job {
            effect,
            clip: &clip,
            frame,
            fade: 1.0,
            place: clip.placement(),
            layer: Layer::Both,
            target: Target::Own(OwnAt {
                clock: frame,
                place: clip.placement(),
                actor: 0,
            }),
            actor: None,
        };
        threads.paint(&job(2));
        threads.end(effect);
        // the same key again is a new effect: its clip goes with its first job, so it paints
        threads.paint(&job(5));
        assert_eq!(wait(&mut threads, 2).len(), 2);
        Ok(())
    }

    /// How long `threads` threads take to paint one window frame's layers of `n` fires (median of 60 frames,
    /// every effect at a new frame each time): under 16.7 ms they keep up at 60 fps.
    #[test]
    #[ignore = "a measurement: run in release with --ignored --nocapture"]
    fn bench_threads_keep_up() -> Result<(), Box<dyn std::error::Error>> {
        let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
        let n_threads = (cores / 2).max(1);
        println!("| fires | {n_threads} threads, ms to paint a frame's layers |");
        println!("|---:|---:|");
        for n in [10usize, 25, 50, 100] {
            let clips: Vec<Clip> = (0..n)
                .filter_map(|i| {
                    let id = ["fire/candle", "fire/campfire", "fire/brazier"][i % 3];
                    let fx = pixfx_effects::effect(id)?;
                    Some(Clip::live(
                        fx,
                        Options {
                            seed: Some(i as u64 + 1),
                            ..Options::default()
                        },
                    ))
                })
                .collect();
            let mut threads = ThreadPainter::new(n_threads)?;
            let mut ms = Vec::new();
            for frame in 1..=60u64 {
                let t = Instant::now();
                for (i, clip) in clips.iter().enumerate() {
                    threads.paint(&Job {
                        effect: EffectKey {
                            scene: 0,
                            handle: i as u32 + 1,
                        },
                        clip,
                        frame,
                        fade: 1.0,
                        place: clip.placement(),
                        layer: Layer::Both,
                        target: Target::Own(OwnAt {
                            clock: frame,
                            place: clip.placement(),
                            actor: 0,
                        }),
                        actor: None,
                    });
                }
                assert_eq!(wait(&mut threads, n).len(), n);
                ms.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            ms.sort_by(f64::total_cmp);
            println!(
                "| {n} | {:.2} |",
                ms.get(ms.len() / 2).copied().unwrap_or(0.0)
            );
        }
        Ok(())
    }
}
