//! The identity of a frame an effect shows. A frame is a pure function of the effect, its options, the margin
//! round its canvas, where it is placed on that canvas and the frame index (live play shows the rendered frames,
//! [`crate::Instance`]), so hosts share one drawn frame among every effect showing the same one.
//!
//! [`ClipKey`] encodes everything but the frame index as canonical bytes: each number by its bits, each list with
//! its length, in a fixed order. The encoder destructures [`Options`] and [`Placement`], so a new field does not
//! compile until it is encoded.

use std::fmt;

use crate::effect::{OptValue, Options, Placement};
use pixfx_draw::color::Rgba;

/// Bumped whenever the encoding changes, so keys written by an older build (rendered clips) never match.
const VERSION: u8 = 2;

/// One effect with one set of options and margin, placed one way: its frames are the same wherever it plays.
/// Shown as lowercase hex, the form hosts pass to JS and write into rendered clips.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClipKey(Vec<u8>);

impl ClipKey {
    /// The key of effect `id` with `opts`, `margin` empty pixels round its canvas, drawn as `place` says.
    pub fn new(id: &str, opts: &Options, margin: u32, place: Placement) -> Self {
        let mut w = Writer(vec![VERSION]);
        w.str(id);
        w.u32(margin);
        w.options(opts);
        w.placement(place);
        Self(w.0)
    }
}

impl fmt::Display for ClipKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl std::str::FromStr for ClipKey {
    type Err = String;

    /// A key from its [`fmt::Display`] form; a key of another encoding version is an error.
    fn from_str(hex: &str) -> Result<Self, String> {
        let bad = || format!("{hex:?} is not a frame key");
        if !hex.len().is_multiple_of(2) {
            return Err(bad());
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| {
                hex.get(i..i + 2)
                    .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                    .ok_or_else(bad)
            })
            .collect::<Result<Vec<u8>, String>>()?;
        match bytes.first() {
            Some(&VERSION) => Ok(Self(bytes)),
            Some(v) => Err(format!(
                "frame key of version {v}, this build reads {VERSION}"
            )),
            None => Err(bad()),
        }
    }
}

/// A frame an effect shows: frame `index` of the clip `key`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameId {
    pub key: ClipKey,
    pub index: u64,
}

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.u64(v.to_bits());
    }
    fn bool(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    fn len(&mut self, n: usize) {
        self.u64(n as u64);
    }
    fn str(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn rgba(&mut self, c: Rgba) {
        self.0.extend_from_slice(&[c.0, c.1, c.2, c.3]);
    }
    fn ramp(&mut self, r: &[Rgba]) {
        self.len(r.len());
        for &c in r {
            self.rgba(c);
        }
    }

    fn options(&mut self, o: &Options) {
        let Options {
            x,
            y,
            flip,
            angle,
            seed,
            light,
            palette,
            extra,
        } = o;
        self.f64(*x);
        self.f64(*y);
        self.bool(*flip);
        self.f64(*angle);
        match seed {
            None => self.u8(0),
            Some(s) => {
                self.u8(1);
                self.u64(*s);
            }
        }
        self.f64(*light);
        match palette {
            None => self.u8(0),
            Some(r) => {
                self.u8(1);
                self.ramp(r);
            }
        }
        self.len(extra.len());
        // a BTreeMap iterates in key order: the same options always encode the same way
        for (name, value) in extra {
            self.str(name);
            match value {
                OptValue::Num(n) => {
                    self.u8(0);
                    self.f64(*n);
                }
                OptValue::Ramp(r) => {
                    self.u8(1);
                    self.ramp(r);
                }
                OptValue::Color(c) => {
                    self.u8(2);
                    self.rgba(*c);
                }
            }
        }
    }

    fn placement(&mut self, p: Placement) {
        let Placement {
            x,
            y,
            flip,
            angle,
            light,
        } = p;
        self.f64(x);
        self.f64(y);
        self.bool(flip);
        self.f64(angle);
        self.f64(light);
    }
}
