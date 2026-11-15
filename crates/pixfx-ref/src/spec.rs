//! What an effect reference is asked for: the effect's look, the moments its rows show, and the sheet's grid.

use serde::{Deserialize, Serialize};

/// The stick figure every cell holds, a guide to where a character stands and how tall it is: 40 sprite pixels from
/// the top of the head (48) to the feet (88) of a 96-pixel cell, the size of the duel's fighters. In
/// `pixfx_pixel::sheet::parse_bones` form.
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
    /// The moments, top to bottom.
    pub rows: Vec<Row>,
    /// Cells a row.
    #[serde(default = "default_frames")]
    pub frames: u32,
    /// Cell side in sprite pixels.
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
    /// notes than frames, a cell and a scale above 0.
    pub fn parse(json: &str) -> Result<Spec, String> {
        let spec: Spec = serde_json::from_str(json).map_err(|e| format!("spec: {e}"))?;
        spec.check()?;
        Ok(spec)
    }

    fn check(&self) -> Result<(), String> {
        let blank = |s: &str| s.trim().is_empty();
        if blank(&self.id) || blank(&self.look) || blank(&self.subject) {
            return Err(format!("spec {}: needs an id, a look and a subject", self.id));
        }
        if self.rows.is_empty() {
            return Err(format!("spec {}: needs at least one row", self.id));
        }
        if let Some(r) = self.rows.iter().find(|r| blank(&r.name) || blank(&r.text)) {
            return Err(format!("spec {}: row \"{}\" needs a name and a text", self.id, r.name));
        }
        if let Some(r) = self.rows.iter().find(|r| self.frames_of(r) == 0) {
            return Err(format!("spec {}: row {} needs at least 1 frame", self.id, r.name));
        }
        if let Some(r) = self.rows.iter().find(|r| r.notes.len() > self.frames_of(r) as usize) {
            return Err(format!("spec {}: row {} has more notes than frames", self.id, r.name));
        }
        if self.cols() < 2 {
            return Err(format!("spec {}: needs a row of at least 2 frames", self.id));
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

    /// Columns of the sheet: the longest row.
    pub fn cols(&self) -> u32 {
        self.rows.iter().map(|r| self.frames_of(r)).max().unwrap_or(0)
    }

    /// The file-name form of the id: `shield/golden-bell` is `shield_golden-bell`.
    pub fn slug(&self) -> String {
        self.id.replace('/', "_")
    }
}
