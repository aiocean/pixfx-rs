//! A grid as text the agent reads cell by cell, edits and hands back: [`text`] writes it, [`parse`] reads it.

use core::fmt;

use pixfx_draw::color::{hex, to_hex};

use crate::{Map, Rgba};

/// The letters [`text`] gives palette colours, in order; an empty cell is `.`.
pub const LEGEND: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// A palette with more colours than [`LEGEND`] has letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooManyColors(pub usize);

impl fmt::Display for TooManyColors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} colours, more than the {} letters a text grid has: reduce the colours first",
            self.0,
            LEGEND.len()
        )
    }
}

/// `map` as text: a head line (size, colours), one legend line per `palette` colour (`a #rrggbb`), then the grid
/// with its column numbers above (tens, then units) and its row number before each row, so every cell can be named
/// by (x, y). `.` is an empty cell. [`parse`] reads it back.
pub fn text(map: &Map, palette: &[Rgba]) -> Result<String, TooManyColors> {
    text_named(map, palette, &[])
}

/// [`text`] with a name on each legend line that has one (`a robe0 #f4efe2`), so the agent reads what a colour is
/// for; a colour past the end of `names`, or with an empty name, gets none (`a #f4efe2`). A name is one word:
/// whitespace in it is dropped.
pub fn text_named(map: &Map, palette: &[Rgba], names: &[&str]) -> Result<String, TooManyColors> {
    if palette.len() > LEGEND.len() {
        return Err(TooManyColors(palette.len()));
    }
    let mut out = format!("{}x{}, {} colours, . empty\n", map.w, map.h, palette.len());
    for (i, (letter, &c)) in LEGEND.chars().zip(palette).enumerate() {
        let name: String = names
            .get(i)
            .map(|n| n.chars().filter(|c| !c.is_whitespace()).collect())
            .unwrap_or_default();
        if name.is_empty() {
            out.push_str(&format!("{letter} {}\n", to_hex(c)));
        } else {
            out.push_str(&format!("{letter} {name} {}\n", to_hex(c)));
        }
    }
    let pad = map.h.saturating_sub(1).to_string().len();
    let digits = |f: fn(u32) -> u32| -> String {
        (0..map.w).map(|x| char::from(b'0' + f(x) as u8)).collect()
    };
    if map.w > 10 {
        out.push_str(&format!("{:pad$} {}\n", "", digits(|x| x / 10 % 10)));
    }
    out.push_str(&format!("{:pad$} {}\n", "", digits(|x| x % 10)));
    let letters: Vec<char> = LEGEND.chars().collect();
    for y in 0..map.h {
        let row: String = (0..map.w)
            .map(|x| {
                map.get(i64::from(x), i64::from(y))
                    .and_then(|i| letters.get(i).copied())
                    .unwrap_or('.')
            })
            .collect();
        out.push_str(&format!("{y:>pad$} {row}\n"));
    }
    Ok(out)
}

/// Why [`parse`] refused a text: the 1-based `line` of the text and what is wrong there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// A grid written by [`text`] or [`text_named`] (and perhaps edited) back to its map and palette:
/// `parse(&text(m, p)?) == (m, p)` for an opaque palette. Read are the head line `WxH, ...`, the legend lines
/// `<letter> [name] #rrggbb` (the palette, in their order; names are for the reader and dropped) and the rows
/// `<row> <cells>`, top to bottom. The column-number lines (one word of digits), the row numbers and blank lines
/// are skipped, so an agent need not keep them right. Refused, naming the line: a letter outside the legend (with
/// its row and column), a row of the wrong width, the wrong number of rows, a legend letter used twice.
pub fn parse(text: &str) -> Result<(Map, Vec<Rgba>), ParseError> {
    let err = |line: usize, message: String| ParseError { line, message };
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty());

    let (head_line, head) = lines
        .next()
        .ok_or_else(|| err(1, "empty text, expected a head line `WxH, ...`".into()))?;
    let size = head.split(',').next().unwrap_or_default().trim();
    let (w, h) = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?)))
        .ok_or_else(|| {
            err(
                head_line,
                format!("head line `{head}` does not start with `WxH`"),
            )
        })?;

    let mut palette = Vec::new();
    let mut letters: Vec<char> = Vec::new();
    let mut map = Map::empty(w, h);
    let mut y: u32 = 0;
    for (line, l) in lines {
        let words: Vec<&str> = l.split_whitespace().collect();
        if let Some((letter, colour)) = legend_line(&words) {
            if y > 0 {
                return Err(err(line, format!("legend line `{l}` after the grid rows")));
            }
            if letter == '.' || letters.contains(&letter) {
                return Err(err(
                    line,
                    format!("legend letter `{letter}` is `.` or given twice"),
                ));
            }
            if letters.len() >= LEGEND.len() {
                return Err(err(line, TooManyColors(letters.len() + 1).to_string()));
            }
            let c = hex(colour)
                .map_err(|_| err(line, format!("`{colour}` is not a #rrggbb colour")))?;
            letters.push(letter);
            palette.push(c);
            continue;
        }
        match words.as_slice() {
            [digits] if digits.bytes().all(|b| b.is_ascii_digit()) => {}
            [_, cells] => {
                if y >= h {
                    return Err(err(line, format!("a row past the {h} the head line gives")));
                }
                let n = cells.chars().count();
                if n != w as usize {
                    return Err(err(
                        line,
                        format!("row {y} has {n} cells, the head line gives {w}"),
                    ));
                }
                for (x, ch) in cells.chars().enumerate() {
                    if ch == '.' {
                        continue;
                    }
                    let i = letters.iter().position(|&l| l == ch).ok_or_else(|| {
                        err(
                            line,
                            format!("row {y}, column {x}: `{ch}` is not in the legend"),
                        )
                    })?;
                    if let Some(cell) = map.cells.get_mut(y as usize * w as usize + x) {
                        *cell = Some(i);
                    }
                }
                y += 1;
            }
            _ => {
                return Err(err(
                    line,
                    format!(
                        "`{l}` is not a legend line, a column-number line or a row `<y> <cells>`"
                    ),
                ));
            }
        }
    }
    if y != h {
        return Err(err(
            text.lines().count().max(1),
            format!("{y} rows, the head line gives {h}"),
        ));
    }
    Ok((map, palette))
}

/// A legend line's letter and colour: a one-character first word and a last word starting with `#`.
fn legend_line<'a>(words: &[&'a str]) -> Option<(char, &'a str)> {
    let (first, rest) = words.split_first()?;
    let last = rest.last()?;
    let mut chars = first.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if last.starts_with('#') => Some((c, *last)),
        _ => None,
    }
}
