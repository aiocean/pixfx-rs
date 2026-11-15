//! The prompt sent with the template: what to draw in the cells, and how.
//!
//! The words are the ones that worked on the xianxia shields (`examples/specs/shield/`): the figure is named as a
//! guide only and the model is told not to draw it, or it draws a character the effect then has to be cut from; and
//! the background is asked for transparent, so each pixel comes back with how much of it is the effect.

use crate::Spec;

const HEAD: &str = "The first image is a sheet of labelled cells, each holding a magenta stick figure. The figure is \
only a guide to size and place: it shows where a character stands and how tall it is. Fill every cell with one frame \
of the same pixel art effect, sized and placed around where the figure stands. Do not draw the figure or any \
character: leave the place where it stands empty, showing only the effect on a transparent background. The label \
above each cell says which moment it shows; read the rows top to bottom and the frames left to right.";

const STYLE: &str = "Draw it in the style of the second image: hard square pixels, ordered dithering for soft edges, \
bright glow. The background is fully transparent everywhere, inside and outside the cells: no dark fill, no \
backdrop; the glow fades into transparency. Keep only the labels opaque.";

/// The prompt for `spec`. It names two images, sent in this order: the template ([`crate::template::render`]),
/// then a style reference (a frame of an existing pixfx effect sets the pixel size and the look).
///
/// A row reads `- INIT row: <text>.`, with `(<n> frames)` after its name when it has its own frame count, and a
/// `  - frame <k>: <note>.` line under it for each note; the spec's rules follow the rows, a line each.
pub fn prompt(spec: &Spec) -> String {
    let mut rows: Vec<String> = Vec::new();
    for r in &spec.rows {
        let count = r.frames.map(|n| format!(" ({n} frames)")).unwrap_or_default();
        rows.push(format!("- {} row{count}: {}.", r.name, r.text));
        for (k, note) in r.notes.iter().enumerate() {
            if !note.trim().is_empty() {
                rows.push(format!("  - frame {}: {}.", k + 1, note.trim().trim_end_matches('.')));
            }
        }
    }
    let rules = if spec.rules.is_empty() { String::new() } else { format!("\n\n{}", spec.rules.join("\n")) };
    format!(
        "{HEAD}\n\nThe effect: {}\n\n{}{rules}\n\nKeep the cell layout and the labels unchanged and remove the magenta figure; \
draw only inside the cells, with the {} the same size and in the same place in every frame. {STYLE}",
        spec.look,
        rows.join("\n"),
        spec.subject
    )
}
