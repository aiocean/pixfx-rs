//! Two pixel fonts: port of pixelfx `src/core/font.js`. [`MINI`] (3x5) for small numbers and tags, [`BIG`] (5x7)
//! for words like LEVEL UP. Glyphs are trimmed to their inked columns, so text sets proportionally (an I is
//! narrower than an M). [`crate::Painter::text`] draws with them; [`spell`]-style layouts read [`Font::glyph`].

/// A pixel font (pixelfx `FONTS.mini` / `FONTS.big`): glyph rows `h` tall, `space` columns for a character it has
/// no glyph for, `gap` empty columns between characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Font {
    pub h: u32,
    pub space: u32,
    pub gap: u32,
    glyphs: &'static [(char, &'static str)],
}

/// One glyph: `w` columns of `h` rows, read with [`Glyph::ink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub w: u32,
    pub h: u32,
    rows: &'static str,
}

impl Glyph {
    /// Whether the font pixel at column `c`, row `r` is inked (pixelfx `g.rows[r][c]`); false outside the glyph.
    pub fn ink(&self, c: u32, r: u32) -> bool {
        self.rows
            .split('|')
            .nth(r as usize)
            .and_then(|row| row.as_bytes().get(c as usize))
            .is_some_and(|&b| b == b'#')
    }
}

impl Font {
    /// The glyph for `ch`, or for its upper case when it has none (pixelfx `F.glyphs[ch] ?? F.glyphs[ch.toUpperCase()]`).
    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        let find = |c: char| {
            self.glyphs
                .iter()
                .find(|(k, _)| *k == c)
                .map(|&(_, rows)| Glyph {
                    w: rows.split('|').next().map_or(0, |r| r.len() as u32),
                    h: self.h,
                    rows,
                })
        };
        find(ch).or_else(|| {
            // JS `toUpperCase` of one code point; a character that upper-cases to several has no glyph
            let mut up = ch.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => find(u),
                _ => None,
            }
        })
    }

    /// Width in font pixels of `s` before scaling (pixelfx `textWidth`).
    pub fn width(&self, s: &str) -> u32 {
        let w: u32 = s
            .chars()
            .map(|ch| self.glyph(ch).map_or(self.space, |g| g.w) + self.gap)
            .sum();
        w.saturating_sub(self.gap)
    }

    /// Calls `plot(x, y)` for every inked font pixel of `s`, the text's top-left at (0, 0) (pixelfx `rasterText`).
    pub fn raster(&self, s: &str, mut plot: impl FnMut(u32, u32)) {
        let mut x = 0;
        for ch in s.chars() {
            let Some(g) = self.glyph(ch) else {
                x += self.space + self.gap;
                continue;
            };
            for (r, row) in g.rows.split('|').enumerate() {
                for (c, b) in row.bytes().take(g.w as usize).enumerate() {
                    if b == b'#' {
                        plot(x + c as u32, r as u32);
                    }
                }
            }
            x += g.w + self.gap;
        }
    }
}

/// The 3x5 font (pixelfx `FONTS.mini`).
pub const MINI: Font = Font {
    h: 5,
    space: 2,
    gap: 1,
    glyphs: &MINI_GLYPHS,
};

/// The 5x7 font (pixelfx `FONTS.big`).
pub const BIG: Font = Font {
    h: 7,
    space: 3,
    gap: 1,
    glyphs: &BIG_GLYPHS,
};

const MINI_GLYPHS: [(char, &str); 55] = [
    ('A', ".#.|#.#|###|#.#|#.#"),
    ('B', "##.|#.#|##.|#.#|##."),
    ('C', ".##|#..|#..|#..|.##"),
    ('D', "##.|#.#|#.#|#.#|##."),
    ('E', "###|#..|##.|#..|###"),
    ('F', "###|#..|##.|#..|#.."),
    ('G', ".##|#..|#.#|#.#|.##"),
    ('H', "#.#|#.#|###|#.#|#.#"),
    ('I', "###|.#.|.#.|.#.|###"),
    ('J', "..#|..#|..#|#.#|.#."),
    ('K', "#.#|#.#|##.|#.#|#.#"),
    ('L', "#..|#..|#..|#..|###"),
    ('M', "#.#|###|###|#.#|#.#"),
    ('N', "##.|#.#|#.#|#.#|#.#"),
    ('O', ".#.|#.#|#.#|#.#|.#."),
    ('P', "##.|#.#|##.|#..|#.."),
    ('Q', ".#.|#.#|#.#|##.|.##"),
    ('R', "##.|#.#|##.|#.#|#.#"),
    ('S', ".##|#..|.#.|..#|##."),
    ('T', "###|.#.|.#.|.#.|.#."),
    ('U', "#.#|#.#|#.#|#.#|###"),
    ('V', "#.#|#.#|#.#|#.#|.#."),
    ('W', "#.#|#.#|###|###|#.#"),
    ('X', "#.#|#.#|.#.|#.#|#.#"),
    ('Y', "#.#|#.#|.#.|.#.|.#."),
    ('Z', "###|..#|.#.|#..|###"),
    ('0', "###|#.#|#.#|#.#|###"),
    ('1', ".#.|##.|.#.|.#.|###"),
    ('2', "##.|..#|.#.|#..|###"),
    ('3', "##.|..#|.#.|..#|##."),
    ('4', "#.#|#.#|###|..#|..#"),
    ('5', "###|#..|##.|..#|##."),
    ('6', ".##|#..|###|#.#|###"),
    ('7', "###|..#|.#.|.#.|.#."),
    ('8', "###|#.#|###|#.#|###"),
    ('9', "###|#.#|###|..#|##."),
    ('+', "...|.#.|###|.#.|..."),
    ('-', "...|...|###|...|..."),
    ('!', "#|#|#|.|#"),
    ('?', "##.|..#|.#.|...|.#."),
    ('.', ".|.|.|.|#"),
    (',', "..|..|..|.#|#."),
    (':', ".|#|.|#|."),
    ('/', "..#|..#|.#.|#..|#.."),
    ('%', "#.#|..#|.#.|#..|#.#"),
    ('x', "...|#.#|.#.|#.#|..."),
    ('*', "#.#|.#.|#.#|...|..."),
    ('\'', "#|#|.|.|."),
    ('(', ".#|#.|#.|#.|.#"),
    (')', "#.|.#|.#|.#|#."),
    ('<', "..#|.#.|#..|.#.|..#"),
    ('>', "#..|.#.|..#|.#.|#.."),
    ('=', "...|###|...|###|..."),
    ('_', "...|...|...|...|###"),
    ('#', "#.#|###|#.#|###|#.#"),
];

const BIG_GLYPHS: [(char, &str); 54] = [
    ('A', ".###.|#...#|#...#|#####|#...#|#...#|#...#"),
    ('B', "####.|#...#|#...#|####.|#...#|#...#|####."),
    ('C', ".###.|#...#|#....|#....|#....|#...#|.###."),
    ('D', "####.|#...#|#...#|#...#|#...#|#...#|####."),
    ('E', "#####|#....|#....|####.|#....|#....|#####"),
    ('F', "#####|#....|#....|####.|#....|#....|#...."),
    ('G', ".###.|#...#|#....|#.###|#...#|#...#|.####"),
    ('H', "#...#|#...#|#...#|#####|#...#|#...#|#...#"),
    ('I', "###|.#.|.#.|.#.|.#.|.#.|###"),
    ('J', "..###|...#.|...#.|...#.|...#.|#..#.|.##.."),
    ('K', "#...#|#..#.|#.#..|##...|#.#..|#..#.|#...#"),
    ('L', "#....|#....|#....|#....|#....|#....|#####"),
    ('M', "#...#|##.##|#.#.#|#.#.#|#...#|#...#|#...#"),
    ('N', "#...#|#...#|##..#|#.#.#|#..##|#...#|#...#"),
    ('O', ".###.|#...#|#...#|#...#|#...#|#...#|.###."),
    ('P', "####.|#...#|#...#|####.|#....|#....|#...."),
    ('Q', ".###.|#...#|#...#|#...#|#.#.#|#..#.|.##.#"),
    ('R', "####.|#...#|#...#|####.|#.#..|#..#.|#...#"),
    ('S', ".####|#....|#....|.###.|....#|....#|####."),
    ('T', "#####|..#..|..#..|..#..|..#..|..#..|..#.."),
    ('U', "#...#|#...#|#...#|#...#|#...#|#...#|.###."),
    ('V', "#...#|#...#|#...#|#...#|#...#|.#.#.|..#.."),
    ('W', "#...#|#...#|#...#|#.#.#|#.#.#|#.#.#|.#.#."),
    ('X', "#...#|#...#|.#.#.|..#..|.#.#.|#...#|#...#"),
    ('Y', "#...#|#...#|.#.#.|..#..|..#..|..#..|..#.."),
    ('Z', "#####|....#|...#.|..#..|.#...|#....|#####"),
    ('0', ".###.|#...#|#..##|#.#.#|##..#|#...#|.###."),
    ('1', ".#.|##.|.#.|.#.|.#.|.#.|###"),
    ('2', ".###.|#...#|....#|...#.|..#..|.#...|#####"),
    ('3', "#####|...#.|..#..|...#.|....#|#...#|.###."),
    ('4', "...#.|..##.|.#.#.|#..#.|#####|...#.|...#."),
    ('5', "#####|#....|####.|....#|....#|#...#|.###."),
    ('6', "..##.|.#...|#....|####.|#...#|#...#|.###."),
    ('7', "#####|....#|...#.|..#..|.#...|.#...|.#..."),
    ('8', ".###.|#...#|#...#|.###.|#...#|#...#|.###."),
    ('9', ".###.|#...#|#...#|.####|....#|...#.|.##.."),
    ('!', "#|#|#|#|#|.|#"),
    ('?', ".###.|#...#|....#|...#.|..#..|.....|..#.."),
    ('+', ".....|..#..|..#..|#####|..#..|..#..|....."),
    ('-', "....|....|....|####|....|....|...."),
    ('.', ".|.|.|.|.|.|#"),
    (',', "..|..|..|..|..|.#|#."),
    (':', ".|.|#|.|.|#|."),
    ('/', "....#|....#|...#.|..#..|.#...|#....|#...."),
    ('%', "##...|##..#|...#.|..#..|.#...|#..##|...##"),
    ('x', ".....|.....|#...#|.#.#.|..#..|.#.#.|#...#"),
    ('*', ".....|..#..|#.#.#|.###.|#.#.#|..#..|....."),
    ('\'', "#|#|.|.|.|.|."),
    ('<', "...#|..#.|.#..|#...|.#..|..#.|...#"),
    ('>', "#...|.#..|..#.|...#|..#.|.#..|#..."),
    ('=', "....|....|####|....|####|....|...."),
    ('(', "..#|.#.|#..|#..|#..|.#.|..#"),
    (')', "#..|.#.|..#|..#|..#|.#.|#.."),
    ('#', ".#.#.|.#.#.|#####|.#.#.|#####|.#.#.|.#.#."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_is_a_full_box() {
        for f in [MINI, BIG] {
            for &(ch, rows) in f.glyphs {
                let rows: Vec<&str> = rows.split('|').collect();
                assert_eq!(rows.len() as u32, f.h, "glyph {ch}");
                assert!(rows.iter().all(|r| r.len() == rows[0].len()), "glyph {ch}");
            }
        }
    }

    #[test]
    fn width_and_lookup_follow_pixelfx() {
        // node: textWidth("Lv 12!", "mini") = 20, textWidth("POW!", "big") = 19, textWidth("", "mini") = 0
        assert_eq!(MINI.width("Lv 12!"), 20);
        assert_eq!(BIG.width("POW!"), 19);
        assert_eq!(MINI.width(""), 0);
        assert_eq!(MINI.glyph('a'), MINI.glyph('A'));
        assert_ne!(MINI.glyph('x'), MINI.glyph('X'));
        assert!(MINI.glyph('~').is_none());
        let g = MINI
            .glyph('I')
            .map(|g| (g.w, g.ink(0, 0), g.ink(0, 1), g.ink(1, 1), g.ink(3, 0)));
        assert_eq!(g, Some((3, true, false, true, false)));
    }
}
