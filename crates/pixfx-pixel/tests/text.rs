// A grid as text and back: what `text` writes, `parse` reads to the same map and palette, and an edit that breaks
// the grid is refused at the cell it breaks.
use pixfx_pixel::{LEGEND, Map, Rgba, parse, text, text_named};

/// A fixed sequence of numbers (xorshift), so every run sees the same maps.
struct Seq(u32);

impl Seq {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// Map `i` of 50: from 1x1 up to 70x12, with up to 62 colours, some cells empty.
fn case(i: u32, seq: &mut Seq) -> (Map, Vec<Rgba>) {
    let (w, h) = (1 + i * 69 / 49, 1 + i * 11 / 49);
    let n = (i * 62 / 49) as usize;
    let palette: Vec<Rgba> = (0..n)
        .map(|_| {
            let v = seq.next();
            Rgba(v as u8, (v >> 8) as u8, (v >> 16) as u8, 255)
        })
        .collect();
    let cells = (0..w * h)
        .map(|_| {
            let k = seq.next() as usize % (n + 1);
            (k < n).then_some(k)
        })
        .collect();
    (Map { w, h, cells }, palette)
}

#[test]
fn text_parses_back_to_the_same_map_and_palette() {
    let mut seq = Seq(0x9e37_79b9);
    for i in 0..50 {
        let (map, palette) = case(i, &mut seq);
        let out = text(&map, &palette).map_err(|e| e.to_string());
        let back = out
            .as_deref()
            .map_err(String::clone)
            .and_then(|t| parse(t).map_err(|e| e.to_string()));
        assert_eq!(back, Ok((map.clone(), palette.clone())), "map {i}");
        // the same text twice: nothing in it depends on the run
        assert_eq!(text(&map, &palette).map_err(|e| e.to_string()), out);
    }
}

#[test]
fn named_legends_name_each_colour_and_parse_back_the_same() {
    let map = Map {
        w: 2,
        h: 1,
        cells: vec![Some(0), Some(1)],
    };
    let palette = [Rgba(0xf4, 0xef, 0xe2, 255), Rgba(0x20, 0x10, 0x30, 255)];
    let out = text_named(&map, &palette, &["robe0"]).unwrap_or_default();
    assert_eq!(
        out,
        "2x1, 2 colours, . empty\na robe0 #f4efe2\nb #201030\n  01\n0 ab\n"
    );
    assert_eq!(parse(&out), Ok((map, palette.to_vec())));

    let mut seq = Seq(7);
    let names: Vec<String> = (0..LEGEND.len()).map(|i| format!("part{i}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    for i in 0..50 {
        let (map, palette) = case(i, &mut seq);
        let out = text_named(&map, &palette, &names).unwrap_or_default();
        assert_eq!(parse(&out), Ok((map, palette)), "map {i}");
    }
}

#[test]
fn parse_skips_column_numbers_and_row_numbers() {
    // the numbers an agent left wrong or dropped do not matter; the rows' order does
    let edited = "3x2, 2 colours, . empty\na #ff0000\nb #0000ff\n\n  999\n7 a.b\n0 .b.\n";
    let bare = "3x2\na #ff0000\nb #0000ff\n0 a.b\n0 .b.\n";
    let want = Map {
        w: 3,
        h: 2,
        cells: vec![Some(0), None, Some(1), None, Some(1), None],
    };
    let palette = vec![Rgba(255, 0, 0, 255), Rgba(0, 0, 255, 255)];
    assert_eq!(parse(edited), Ok((want.clone(), palette.clone())));
    assert_eq!(parse(bare), Ok((want, palette)));
}

#[test]
fn a_letter_outside_the_legend_is_refused_at_its_row_and_column() {
    let t = "3x2, 2 colours, . empty\na #ff0000\nb #0000ff\n  012\n0 a.b\n1 .q.\n";
    let e = parse(t).map_err(|e| (e.line, e.message));
    assert!(
        matches!(&e, Err((6, m)) if m.contains("row 1, column 1") && m.contains('q')),
        "{e:?}"
    );
}

#[test]
fn a_row_of_the_wrong_width_or_a_missing_row_is_refused() {
    let wide = parse("3x2\na #ff0000\n0 a.a.\n1 ...\n").map_err(|e| (e.line, e.message));
    assert!(
        matches!(&wide, Err((3, m)) if m.contains("row 0 has 4 cells")),
        "{wide:?}"
    );
    assert!(parse("3x2\na #ff0000\n0 a.a\n").is_err());
    assert!(parse("3x1\na #ff0000\n0 a.a\n1 a.a\n").is_err());
    assert!(parse("").is_err());
    assert!(parse("wide\n0 ...\n").is_err());
    // a letter given twice in the legend would make cells ambiguous
    assert!(parse("1x1\na #ff0000\na #00ff00\n0 a\n").is_err());
}
