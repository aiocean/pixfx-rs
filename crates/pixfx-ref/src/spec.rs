//! What an effect reference is asked for: the effect's look, its parts, the moments its rows show, and the sheet's
//! grid.

use serde::{Deserialize, Serialize};

/// The stick figure every moment cell holds, a guide to where a character stands and how tall it is: 40 sprite pixels
/// from the top of the head (48) to the feet (88) of a 96-pixel cell, the size of the duel's fighters. In
/// `pixfx_pixel::sheet::parse_bones` form, in a 96-pixel cell's coordinates: a cell of another size scales it.
pub const FIGURE: &str = "\
46,48 50,48
50,48 50,54
50,54 46,54
46,54 46,48
48,54 48,72
48,58 42,68
48,58 54,68
48,72 44,88
48,72 52,88
";

/// One row of the sheet: a moment of the effect, shown in `frames` cells left to right.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// The row's label, upper case by habit: `INIT`, `IDLE`, `HIT`, `BREAK`.
    pub name: String,
    /// A few words after the first cell's label: `shield forms`.
    pub caption: String,
    /// What happens in the row, for the prompt: one sentence, no full stop at the end.
    pub text: String,
    /// Cells in this row when it needs more or fewer than the spec's `frames`: a slow build-up, a short blow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u32>,
    /// What one frame must show, by frame from the first; an empty note says nothing about its frame. Use it where
    /// the model takes a shortcut: left alone it draws the first frame of a build-up half formed, so say "only a few
    /// faint specks, under 20 pixels" for frame 1 and the like for the last frames of an ending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// One part of the effect, drawn alone in the sheet's PARTS row so it can be drawn again piece by piece: the main
/// object, a piece of it (one petal), the small effects round it, its light.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// The part's label, upper case by habit: `MAIN`, `PETAL`, `FX`, `LIGHT`.
    pub name: String,
    /// What the cell holds, for the prompt: one sentence, no full stop at the end.
    pub text: String,
    /// Where the part turns when it moves, for the prompt (`its base, where it joins the pad`): the model marks that
    /// point with one [`PIVOT`] square, which `extract` reads off and removes. Empty for no mark.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pivot: String,
    /// The shapes the part takes in the effect, a cell each in a row of its own under the PARTS row
    /// (`PETAL 1/6`...): a part that turns, bends or curls is drawn in each form the moments need, so none has to be
    /// guessed (`upright, closed`, `leaning out, tip curled in`, `tumbling, seen edge-on`). Empty for none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<String>,
}

/// One layer of a moment: each moment row is drawn once per layer, frame for frame, each holding only its layer
/// (`SOLID`: the shapes; `FX`: the small effects; `LIGHT`: the glow), so laid over each other the cells of one column
/// make the whole frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    /// The layer's label, upper case by habit: `SOLID`, `FX`, `LIGHT`.
    pub name: String,
    /// What it holds, for the prompt: one sentence, no full stop at the end.
    pub text: String,
    /// Whether the layer is light: when the layers are put back together into the moment's frames, it is added over
    /// the others, as pixfx adds light over colour, not laid over them (`LIGHT`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub light: bool,
}

/// The colour a part's pivot is marked in: pure green, a colour no effect of ours is drawn in.
pub const PIVOT: [u8; 3] = [0, 255, 0];

/// The row name of the parts, first on the sheet; its cells are named `parts-<part>` (`parts-main`).
pub const PARTS: &str = "PARTS";

/// The sheet row of `part`'s forms: `PARTS-PETAL`, its cells `parts-petal-1`...
pub fn forms_row(part: &Part) -> String {
    format!("{PARTS}-{}", part.name)
}

/// Whether a cell's or a sheet row's name is a part's (`parts-main`, `parts-petal-2`), not a frame of a moment.
pub fn is_part(cell: &str) -> bool {
    cell.to_ascii_lowercase().starts_with("parts")
}

/// An effect reference: what the image model is asked to draw, and on which grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    /// The effect id it becomes: `shield/golden-bell`.
    pub id: String,
    /// A short name for pages: `Golden Bell`.
    pub name: String,
    /// What the effect looks like, for the prompt: a name, then a sentence or two.
    pub look: String,
    /// The thing that keeps its size and place in every frame: `shield`, `bell`.
    pub subject: String,
    /// The parts, drawn alone in a PARTS row above the moments; none for no such row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    /// The moments, top to bottom.
    pub rows: Vec<Row>,
    /// The parts already drawn on the parts sheet that goes with a moment's sheet ([`Spec::moment`]): named in its
    /// prompt, not drawn on it. Set by [`Spec::moment`], never read from JSON.
    #[serde(skip)]
    pub drawn_parts: Vec<Part>,
    /// The layers each moment is drawn in, a sheet row each (`INIT-SOLID`, `INIT-FX`, ...); none for one row a moment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
    /// Whether every cell shows its pixel grid (a thin grey line round every sprite pixel, a darker one every 8)
    /// and the model is asked to draw whole grid squares: the cells then come back at exactly `cell` pixels a side.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub grid: bool,
    /// Cells a row.
    #[serde(default = "default_frames")]
    pub frames: u32,
    /// Cell side in sprite pixels: the resolution the effect is drawn at (64 with a grid, the duel's 96 without).
    #[serde(default = "default_cell")]
    pub cell: u32,
    /// Sprite pixels between cells and round the edge; the labels are written in the gap above each cell.
    #[serde(default = "default_gap")]
    pub gap: u32,
    /// Sheet pixels a sprite pixel.
    #[serde(default = "default_scale")]
    pub scale: u32,
    /// The guide figure in every cell, as bones text; [`FIGURE`] by default.
    #[serde(default = "default_figure")]
    pub figure: String,
    /// Lines for the whole effect, added to the prompt after the rows: how it grows, what it never does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
}

fn default_frames() -> u32 {
    3
}
fn default_cell() -> u32 {
    96
}
fn default_gap() -> u32 {
    14
}
fn default_scale() -> u32 {
    3
}
fn default_figure() -> String {
    FIGURE.to_string()
}

impl Spec {
    /// A spec from its JSON, checked: an id, a look, a subject, at least one row with a name and a text, at least two
    /// frame in every row and a row of at least two (the cells are found again from the spacing of the labels), no more
    /// notes than frames, a name and a text for every part and no moment row named PARTS, a cell and a scale above 0.
    pub fn parse(json: &str) -> Result<Spec, String> {
        let spec: Spec = serde_json::from_str(json).map_err(|e| format!("spec: {e}"))?;
        spec.check()?;
        Ok(spec)
    }

    fn check(&self) -> Result<(), String> {
        let blank = |s: &str| s.trim().is_empty();
        if blank(&self.id) || blank(&self.look) || blank(&self.subject) {
            return Err(format!(
                "spec {}: needs an id, a look and a subject",
                self.id
            ));
        }
        if self.rows.is_empty() {
            return Err(format!("spec {}: needs at least one row", self.id));
        }
        if let Some(r) = self.rows.iter().find(|r| blank(&r.name) || blank(&r.text)) {
            return Err(format!(
                "spec {}: row \"{}\" needs a name and a text",
                self.id, r.name
            ));
        }
        if let Some(p) = self.parts.iter().find(|p| p.forms.iter().any(|f| blank(f))) {
            return Err(format!(
                "spec {}: part {} has a blank form",
                self.id, p.name
            ));
        }
        if let Some(r) = self.rows.iter().find(|r| self.frames_of(r) == 0) {
            return Err(format!(
                "spec {}: row {} needs at least 1 frame",
                self.id, r.name
            ));
        }
        if let Some(p) = self.parts.iter().find(|p| blank(&p.name) || blank(&p.text)) {
            return Err(format!(
                "spec {}: part \"{}\" needs a name and a text",
                self.id, p.name
            ));
        }
        if let Some(l) = self
            .layers
            .iter()
            .find(|l| blank(&l.name) || blank(&l.text))
        {
            return Err(format!(
                "spec {}: layer \"{}\" needs a name and a text",
                self.id, l.name
            ));
        }
        if self
            .rows
            .iter()
            .any(|r| r.name.trim().eq_ignore_ascii_case(PARTS))
        {
            return Err(format!(
                "spec {}: {PARTS} is the parts row's name; give the parts in `parts`",
                self.id
            ));
        }
        if let Some(r) = self
            .rows
            .iter()
            .find(|r| r.notes.len() > self.frames_of(r) as usize)
        {
            return Err(format!(
                "spec {}: row {} has more notes than frames",
                self.id, r.name
            ));
        }
        if self.cols() < 2 {
            return Err(format!(
                "spec {}: needs a row of at least 2 frames",
                self.id
            ));
        }
        if self.cell == 0 || self.scale == 0 {
            return Err(format!("spec {}: cell and scale must be above 0", self.id));
        }
        Ok(())
    }

    /// Cells in `row`: its own `frames`, else the spec's.
    pub fn frames_of(&self, row: &Row) -> u32 {
        row.frames.unwrap_or(self.frames)
    }

    /// Columns of the sheet: the longest row, the parts and form rows included.
    pub fn cols(&self) -> u32 {
        self.sheet_rows().iter().map(|r| r.1).max().unwrap_or(0)
    }

    /// The rows of parts on the sheet: the PARTS row and a row per part with forms, or none.
    pub fn part_rows(&self) -> Vec<(String, u32)> {
        let n = |len: usize| u32::try_from(len).unwrap_or(u32::MAX);
        let head = (!self.parts.is_empty()).then(|| (PARTS.to_string(), n(self.parts.len())));
        head.into_iter()
            .chain(
                self.parts
                    .iter()
                    .filter(|p| !p.forms.is_empty())
                    .map(|p| (forms_row(p), n(p.forms.len()))),
            )
            .collect()
    }

    /// The moment rows of the sheet top to bottom: each moment once, or once per layer as `INIT-SOLID`, with the
    /// moment and its layer.
    pub fn moment_rows(&self) -> Vec<(String, &Row, Option<&Layer>)> {
        self.rows
            .iter()
            .flat_map(|r| {
                let one = self.layers.is_empty().then(|| (r.name.clone(), r, None));
                let each = self
                    .layers
                    .iter()
                    .map(move |l| (format!("{}-{}", r.name, l.name), r, Some(l)));
                one.into_iter().chain(each)
            })
            .collect()
    }

    /// The sheet's rows top to bottom as (name, cells): the parts row first when there are parts, then the moment
    /// rows ([`Spec::moment_rows`]).
    pub fn sheet_rows(&self) -> Vec<(String, u32)> {
        self.part_rows()
            .into_iter()
            .chain(
                self.moment_rows()
                    .into_iter()
                    .map(|(name, r, _)| (name, self.frames_of(r))),
            )
            .collect()
    }

    /// The parts sheet: the PARTS row and the form rows, no moments. Drawn once, before the moments' sheets, which
    /// are drawn from it.
    pub fn parts_sheet(&self) -> Spec {
        Spec {
            rows: Vec::new(),
            ..self.clone()
        }
    }

    /// The sheet of one moment (any case): only its rows, one per layer, with no parts. The parts sheet goes with it
    /// as a reference image, so every moment is built from the same parts; each sheet keeps the resolution of a few
    /// rows.
    pub fn moment(&self, moment: &str) -> Result<Spec, String> {
        let row = self
            .rows
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(moment))
            .ok_or_else(|| format!("spec {}: no moment {moment}", self.id))?;
        Ok(Spec {
            rows: vec![row.clone()],
            parts: Vec::new(),
            drawn_parts: self.parts.clone(),
            ..self.clone()
        })
    }

    /// The file-name form of the id: `shield/golden-bell` is `shield_golden-bell`.
    pub fn slug(&self) -> String {
        self.id.replace('/', "_")
    }
}
