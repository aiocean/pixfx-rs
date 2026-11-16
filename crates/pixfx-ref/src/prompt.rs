//! The prompt sent with the template: what to draw in the cells, and how.
//!
//! The words are the ones that worked. On the xianxia shields (`examples/specs/shield/`): the figure is named as a
//! guide only and the model is told not to draw it, or it draws a character the effect then has to be cut from; and
//! the background is asked for transparent, so each pixel comes back with how much of it is the effect. On the
//! lotus with a grid and parts: whole grid squares in one flat colour, a small palette shared by every cell, the
//! grid lines not drawn, and each part alone, whole, at the size it has in the moments; the model then kept the
//! cells where they were and drew the parts apart.

use crate::Spec;
use crate::spec::PARTS;

const HEAD: &str = "The first image is a sheet of labelled cells, each holding a magenta stick figure. The figure is \
only a guide to size and place: it shows where a character stands and how tall it is. Fill every cell with one frame \
of the same pixel art effect, sized and placed around where the figure stands. Do not draw the figure or any \
character: leave the place where it stands empty, showing only the effect on a transparent background. The label \
above each cell says which moment it shows; read the rows top to bottom and the frames left to right.";

const STYLE: &str = "Draw it in the style of the second image: hard square pixels, ordered dithering for soft edges, \
bright glow. The background is fully transparent everywhere, inside and outside the cells: no dark fill, no \
backdrop; the glow fades into transparency. Keep only the labels opaque.";

const GRID_STYLE: &str = "Draw it in the style of the second image: hard square pixels, bright glow. The background \
is fully transparent everywhere, inside and outside the cells: no dark fill, no backdrop. Keep only the labels opaque.";

/// The prompt for `spec`. It names two images, sent in this order: the template ([`crate::template::render`]),
/// then a style reference (a frame of an existing pixfx effect sets the pixel size and the look).
///
/// A row reads `- INIT row: <text>.`, with `(<n> frames)` after its name when it has its own frame count, and a
/// `  - frame <k>: <note>.` line under it for each note; the spec's rules follow the rows, a line each. A spec with a
/// grid or parts gets [`sheet_prompt`]; one with neither keeps the words the shields were first drawn from.
pub fn prompt(spec: &Spec) -> String {
    if spec.grid
        || !spec.parts.is_empty()
        || !spec.drawn_parts.is_empty()
        || !spec.layers.is_empty()
    {
        return sheet_prompt(spec);
    }
    let rules = rules(spec);
    format!(
        "{HEAD}\n\nThe effect: {}\n\n{}{rules}\n\nKeep the cell layout and the labels unchanged and remove the magenta figure; \
draw only inside the cells, with the {} the same size and in the same place in every frame. {STYLE}",
        spec.look,
        moments(spec),
        spec.subject
    )
}

/// The ruler in grid squares (`compare::art_ruler`): how big the character the effect plays on is, so every
/// object is drawn at its size beside that character, part for part.
fn scale(cell: u32) -> String {
    let r = crate::compare::art_ruler(cell);
    let squares = |n: u32| {
        if n == 1 {
            "1 square".to_string()
        } else {
            format!("{n} squares")
        }
    };
    format!(
        " The character this effect plays on is {} tall on this grid: its head is {} wide, its hand {} and its eye \
{}. Draw everything at that scale, as big as it would be beside that character (a lens as wide as the eye, a sword \
as long as {}), and nothing smaller than one square.",
        squares(r.height),
        squares(r.head),
        squares(r.hand),
        squares(r.eye.0),
        squares(r.blade)
    )
}

/// The prompt for a sheet with a pixel grid, parts, or both: the grid first (whole squares, one flat colour each, a
/// small shared palette, lines not drawn), the effect, the parts each alone (`- PARTS 1, MAIN: <text>.`), then the
/// moments round the figure, built from the parts.
pub fn sheet_prompt(spec: &Spec) -> String {
    let n = spec.cell;
    let grid = if spec.grid {
        format!(
            " Every cell is a grid of {n} by {n} squares: each square is one pixel of the art. Draw pixel art on that \
grid: fill whole squares with one flat colour each, never part of a square, never a colour blended across squares. Use \
a small palette of about 8 colours plus 4 glow colours, the same palette in every cell. The grid lines are a guide \
only: do not draw them, and leave every square you do not fill transparent.{}",
            scale(n)
        )
    } else {
        String::new()
    };
    let parts = if spec.parts.is_empty() {
        String::new()
    } else {
        let mut list: Vec<String> = Vec::new();
        for (i, p) in spec.parts.iter().enumerate() {
            let pivot = if p.pivot.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "; mark {} with one pure green (#00ff00) square{}",
                    p.pivot.trim().trim_end_matches('.'),
                    if p.forms.is_empty() {
                        ""
                    } else {
                        ", in every form"
                    }
                )
            };
            list.push(format!(
                "- {PARTS} {}, {}: {}{pivot}.",
                i + 1,
                p.name,
                p.text.trim().trim_end_matches('.')
            ));
        }
        let forms: Vec<String> = spec
            .parts
            .iter()
            .filter(|p| !p.forms.is_empty())
            .map(|p| {
                let each: Vec<String> = p
                    .forms
                    .iter()
                    .enumerate()
                    .map(|(k, f)| format!("  - {} {}: {}.", p.name, k + 1, f.trim().trim_end_matches('.')))
                    .collect();
                format!(
                    "- the {} row: the same {} in {} forms, the shapes it takes in the effect, each alone and whole, at \
the same size and in the same colours:\n{}",
                    p.name,
                    p.name.to_lowercase(),
                    p.forms.len(),
                    each.join("\n")
                )
            })
            .collect();
        let forms = if forms.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nUnder it, a row for each part that turns, bends or curls:\n{}",
                forms.join("\n")
            )
        };
        format!(
            "\n\nThe top row, {PARTS}, is the effect taken apart, so it can be drawn again piece by piece. Each part is \
drawn alone, whole, at the same size as it appears in the effect, centred in its cell:\n{}{forms}",
            list.join("\n")
        )
    };
    let drawn = drawn(spec);
    let size = if spec.grid { ", the grid size" } else { "" };
    if spec.rows.is_empty() {
        return format!(
            "The first image is a sheet of labelled cells.{grid}\n\nThe effect: {}{parts}{}\n\nKeep the cells{size} and \
the labels unchanged; draw only inside the cells. {GRID_STYLE}",
            spec.look,
            rules(spec)
        );
    }
    let (which, built) = match (spec.parts.is_empty(), drawn.is_empty()) {
        (false, _) => (
            "The other rows are the moments",
            ", built from the parts in the top rows",
        ),
        (true, false) => (
            "The rows are the moments",
            ", built from the parts on the third image",
        ),
        (true, true) => ("The rows are the moments", ""),
    };
    let layers = layers(spec);
    format!(
        "The first image is a sheet of labelled cells.{grid}\n\nThe effect: {}{parts}{drawn}\n\n{which}, read top to bottom and \
frames left to right. Each of their cells holds a magenta stick figure: it is only a guide to where a character \
stands and how tall it is. Do not draw the figure or any character: leave its place empty and draw only the effect \
around it{built}.\n{}{layers}{}\n\nKeep the cells{size} and the labels unchanged and remove the magenta figure; draw only \
inside the cells, with the {} the same size and in the same place in every frame. {GRID_STYLE}",
        spec.look,
        moments(spec),
        rules(spec),
        spec.subject
    )
}

/// What the third image, the parts sheet, holds, for a moment's sheet ([`Spec::moment`]); nothing otherwise.
fn drawn(spec: &Spec) -> String {
    if spec.drawn_parts.is_empty() {
        return String::new();
    }
    let names: Vec<String> = spec
        .drawn_parts
        .iter()
        .map(|p| {
            if p.forms.is_empty() {
                p.name.clone()
            } else {
                format!("{} (in {} forms)", p.name, p.forms.len())
            }
        })
        .collect();
    format!(
        "\n\nThe third image is this effect's parts sheet: {}, each drawn alone. Build every frame only from those \
parts and forms, drawn exactly as they are there, the same shapes, colours and sizes: place them, turn them, overlap \
them and pick the form each needs, but never invent a new shape. In every frame the main object keeps MAIN's \
shape, number of pieces and size, unless its row says it changes; a cracked, broken, burning or fading form \
belongs only in a row that says the effect is struck or ends. Leave out the green pivot marks.",
        names.join(", ")
    )
}

/// How each moment is split into layer rows, or nothing for a spec without layers.
fn layers(spec: &Spec) -> String {
    if spec.layers.is_empty() {
        return String::new();
    }
    let names: Vec<String> = spec
        .rows
        .iter()
        .flat_map(|r| {
            spec.layers
                .iter()
                .map(move |l| format!("{}-{}", r.name, l.name))
        })
        .collect();
    let list: Vec<String> = spec
        .layers
        .iter()
        .map(|l| {
            format!(
                "- {}: only {}.",
                l.name,
                l.text.trim().trim_end_matches('.')
            )
        })
        .collect();
    format!(
        "\n\nEach moment is drawn in {} rows ({}), one per layer, frame for frame: the cells of one column are the \
same instant, and laid over each other they make the whole frame. A layer row holds only its own layer and nothing \
of the others, at exactly the size and place it has in the whole frame:\n{}",
        spec.layers.len(),
        names.join(", "),
        list.join("\n")
    )
}

/// The moment rows, a line each, their notes under them.
fn moments(spec: &Spec) -> String {
    let mut rows: Vec<String> = Vec::new();
    for r in &spec.rows {
        let count = r
            .frames
            .map(|n| format!(" ({n} frames)"))
            .unwrap_or_default();
        rows.push(format!("- {} row{count}: {}.", r.name, r.text));
        for (k, note) in r.notes.iter().enumerate() {
            if !note.trim().is_empty() {
                rows.push(format!(
                    "  - frame {}: {}.",
                    k + 1,
                    note.trim().trim_end_matches('.')
                ));
            }
        }
    }
    rows.join("\n")
}

/// The spec's rules after a blank line, or nothing.
fn rules(spec: &Spec) -> String {
    if spec.rules.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", spec.rules.join("\n"))
    }
}
