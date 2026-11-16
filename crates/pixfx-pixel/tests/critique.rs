// Critique: each check finds its defect at the right cells, a clean sprite and a real fighter pose give no error.
use pixfx_pixel::RgbaImage;
use pixfx_pixel::critique::{Issue, Report, Severity, critique, critique_frames};

fn fill(w: u32, h: u32, f: impl Fn(u32, u32) -> Option<[u8; 4]>) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if let Some(p) = f(x, y) {
                img.set(x, y, p);
            }
        }
    }
    img
}

fn of<'a>(r: &'a Report, check: &str) -> Vec<&'a Issue> {
    r.issues.iter().filter(|i| i.check == check).collect()
}

fn one(r: &Report, check: &str) -> Result<Issue, String> {
    match of(r, check).as_slice() {
        [i] => Ok((*i).clone()),
        other => Err(format!(
            "{check}: {} issues in {:#?}",
            other.len(),
            r.issues
        )),
    }
}

/// A 16x16 block: dark outline, lit from the top-left.
fn clean_sprite() -> RgbaImage {
    fill(16, 16, |x, y| {
        if !(2..14).contains(&x) || !(2..14).contains(&y) {
            None
        } else if x == 2 || y == 2 || x == 13 || y == 13 {
            Some([30, 24, 48, 255])
        } else if x + y < 16 {
            Some([220, 120, 80, 255])
        } else {
            Some([150, 70, 60, 255])
        }
    })
}

#[test]
fn a_clean_sprite_has_no_issue() {
    let r = critique(&clean_sprite(), &[]);
    assert!(r.issues.is_empty(), "{:#?}", r.issues);
    assert_eq!(r.score, 100);
}

#[test]
fn an_empty_picture_is_an_error() {
    let r = critique(&RgbaImage::new(8, 8), &[]);
    let i = one(&r, "empty").map_err(|e| e.to_string());
    assert_eq!(i.map(|i| i.severity), Ok(Severity::Error));
    assert_eq!(r.score, 70);
}

#[test]
fn colour_budget_names_the_least_used_colours() -> Result<(), String> {
    // 100 painted cells: budget round(10 * 1.2) = 12; one base colour and 13 single cells, the two last over budget.
    let img = fill(20, 5, |x, y| {
        let i = x + 1;
        if y == 2 && i <= 13 {
            Some([(i * 18) as u8, (255 - i * 18) as u8, 128, 255])
        } else {
            Some([10, 10, 10, 255])
        }
    });
    let i = one(&critique(&img, &[]), "colour_budget")?;
    assert_eq!(i.cells, vec![(11, 2), (12, 2)]);
    assert_eq!(i.severity, Severity::Warning);
    Ok(())
}

#[test]
fn near_duplicate_points_at_the_rarer_colour() -> Result<(), String> {
    let img = fill(4, 4, |x, y| {
        Some(if (x, y) == (3, 1) {
            [101, 100, 100, 255]
        } else {
            [100, 100, 100, 255]
        })
    });
    let r = critique(&img, &[]);
    assert_eq!(one(&r, "near_duplicate")?.cells, vec![(3, 1)]);
    assert!(of(&r, "orphan").is_empty(), "a near shade is not an orphan");
    Ok(())
}

#[test]
fn semi_transparent_lists_its_cells() -> Result<(), String> {
    let img = fill(4, 4, |x, y| {
        Some([
            90,
            90,
            200,
            if (x, y) == (0, 0) || (x, y) == (2, 3) {
                60
            } else {
                255
            },
        ])
    });
    assert_eq!(
        one(&critique(&img, &[]), "semi_transparent")?.cells,
        vec![(0, 0), (2, 3)]
    );
    Ok(())
}

#[test]
fn an_orphan_is_found_and_intended_silences_it() -> Result<(), String> {
    let img = fill(5, 5, |x, y| {
        Some(if (x, y) == (2, 2) {
            [230, 30, 30, 255]
        } else {
            [120, 120, 120, 255]
        })
    });
    let i = one(&critique(&img, &[]), "orphan")?;
    assert_eq!(i.cells, vec![(2, 2)]);
    assert!(
        i.fix.contains("(2,2)") && i.fix.contains("--intended"),
        "{}",
        i.fix
    );
    assert!(of(&critique(&img, &[(2, 2)]), "orphan").is_empty());
    Ok(())
}

#[test]
fn noise_is_found_on_a_checkerboard() -> Result<(), String> {
    let img = fill(6, 6, |x, y| {
        Some(if (x + y) % 2 == 0 {
            [240, 240, 240, 255]
        } else {
            [20, 20, 20, 255]
        })
    });
    let i = one(&critique(&img, &[]), "noise")?;
    assert!(
        i.cells.contains(&(1, 1)) && !i.cells.contains(&(0, 0)),
        "{:?}",
        i.cells
    );
    Ok(())
}

#[test]
fn weak_outline_lists_light_edge_cells() -> Result<(), String> {
    let img = fill(8, 8, |_, _| Some([200, 200, 200, 255]));
    let i = one(&critique(&img, &[]), "weak_outline")?;
    assert_eq!(i.cells.first(), Some(&(0, 0)));
    assert!(!i.cells.contains(&(3, 3)), "an inner cell is not an edge");
    Ok(())
}

#[test]
fn jaggies_are_the_inner_corners_of_a_staircase() -> Result<(), String> {
    let img = fill(12, 12, |x, y| (x <= y).then_some([30, 30, 30, 255]));
    let i = one(&critique(&img, &[]), "jaggies")?;
    let want: Vec<(u32, u32)> = (0..11).map(|y| (y + 1, y)).collect();
    assert_eq!(i.cells, want);
    Ok(())
}

#[test]
fn pillow_shading_is_light_rising_toward_the_centre() -> Result<(), String> {
    let img = fill(15, 15, |x, y| {
        let d = x.min(y).min(14 - x).min(14 - y);
        let v = (40 + d * 25) as u8;
        Some([v, v, v, 255])
    });
    let i = one(&critique(&img, &[]), "pillow_shading")?;
    assert_eq!(i.cells, vec![(7, 7)]);
    Ok(())
}

#[test]
fn the_score_takes_30_12_and_4() {
    // Light 8x8 square: weak_outline (note) only.
    let r = critique(&fill(8, 8, |_, _| Some([200, 200, 200, 255])), &[]);
    let cost: u32 = r
        .issues
        .iter()
        .map(|i| match i.severity {
            Severity::Error => 30,
            Severity::Warning => 12,
            Severity::Note => 4,
        })
        .sum();
    assert_eq!(r.score, 100 - cost);
}

#[test]
fn issues_are_ordered_by_severity_then_check() {
    let img = fill(6, 6, |x, y| {
        Some(if (x + y) % 2 == 0 {
            [240, 240, 240, 255]
        } else {
            [200, 200, 200, 100]
        })
    });
    let r = critique(&img, &[]);
    let keys: Vec<(Severity, &str)> = r.issues.iter().map(|i| (i.severity, i.check)).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

fn block(x0: u32, y0: u32, s: u32, c: [u8; 4]) -> RgbaImage {
    fill(16, 16, |x, y| {
        ((x0..x0 + s).contains(&x) && (y0..y0 + s).contains(&y)).then_some(c)
    })
}

#[test]
fn frames_find_volume_ground_palette_empty_and_repeated() -> Result<(), String> {
    let a = block(4, 8, 6, [200, 60, 60, 255]); // bottom row 13
    let lifted = block(4, 2, 6, [200, 60, 60, 255]); // bottom row 7
    let big = block(2, 4, 10, [200, 60, 60, 255]); // 100 cells against 36
    let recoloured = fill(16, 16, |x, y| {
        ((4..10).contains(&x) && (8..14).contains(&y)).then_some([
            (x * 30) as u8,
            200,
            (y * 10) as u8,
            255,
        ])
    });
    let empty = RgbaImage::new(16, 16);
    let r = critique_frames(&[a.clone(), a.clone(), lifted, big, recoloured, empty]);

    assert!(one(&r, "repeated_frame")?.message.contains("frame 1"));
    assert!(one(&r, "empty_frame")?.message.contains("frame 5"));
    let v = one(&r, "volume_drift")?;
    assert!(v.message.starts_with("frame 3"), "{}", v.message);
    let g = one(&r, "ground_drift")?;
    assert_eq!(g.cells, vec![(4, 7)]);
    let p = one(&r, "palette_drift")?;
    assert!(p.message.starts_with("frame 4"), "{}", p.message);
    assert_eq!(p.cells.first(), Some(&(4, 8)));
    Ok(())
}

#[test]
fn steady_frames_have_no_issue() {
    let a = block(4, 8, 6, [200, 60, 60, 255]);
    let b = block(5, 8, 6, [200, 60, 60, 255]);
    let r = critique_frames(&[a, b]);
    assert!(r.issues.is_empty(), "{:#?}", r.issues);
}

#[test]
fn a_real_fighter_pose_has_no_error() -> Result<(), String> {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/thunder-idle.png"
    ))
    .map_err(|e| e.to_string())?;
    let img = pixfx_export::decode_png_rgba(&bytes)?;
    let r = critique(&img, &[]);
    for i in &r.issues {
        println!(
            "{:?} {} {:?}\n  {}\n  fix: {}",
            i.severity, i.check, i.cells, i.message, i.fix
        );
    }
    println!("score {}", r.score);
    assert!(
        r.issues.iter().all(|i| i.severity != Severity::Error),
        "{:#?}",
        r.issues
    );
    Ok(())
}

/// A sprite from rows of letters: `i` the ink (darkest), `a` `b` `c` greys from dark to light, `.` empty.
fn rows(rows: &[&str]) -> RgbaImage {
    let grid: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let w = grid.first().map_or(0, Vec::len) as u32;
    fill(w, grid.len() as u32, |x, y| {
        let grey = |v: u8| Some([v, v, v, 255]);
        match grid.get(y as usize).and_then(|r| r.get(x as usize)) {
            Some('i') => grey(10),
            Some('a') => grey(60),
            Some('b') => grey(130),
            Some('c') => grey(220),
            _ => None,
        }
    })
}

fn cells_of(r: &Report, check: &str) -> Vec<(u32, u32)> {
    of(r, check).iter().flat_map(|i| i.cells.clone()).collect()
}

#[test]
fn a_cell_sticking_out_of_the_shape_is_stray_unless_meant() {
    let img = rows(&["i....", ".bb..", ".bb.i", "....."]);
    assert_eq!(
        cells_of(&critique(&img, &[]), "stray"),
        vec![(0, 0), (4, 2)]
    );
    assert_eq!(cells_of(&critique(&img, &[(4, 2)]), "stray"), vec![(0, 0)]);
    let speck = rows(&["bbbbb", "bbibb", "bbbbb"]);
    assert!(
        of(&critique(&speck, &[]), "stray").is_empty(),
        "a speck inside the shape is orphan's, not stray's"
    );
}

#[test]
fn an_l_in_a_one_cell_line_is_a_doubled_corner_but_a_box_corner_is_not() {
    let corner = |r: &[&str]| cells_of(&critique(&rows(r), &[]), "doubled_corner");
    assert_eq!(corner(&["i....", ".ii..", "..i..", "...i."]), vec![(2, 1)]);
    assert!(corner(&["i....", ".i...", "..i..", "...i."]).is_empty());
    assert!(
        corner(&["ii...", ".ii..", "..ii.", "...ii"]).is_empty(),
        "steps all the way are a diagonal two cells wide"
    );
    assert!(
        corner(&["iiiii", "i...i", "i...i", "iiiii"]).is_empty(),
        "a box corner is meant"
    );
    assert_eq!(
        corner(&["iiiii", "....i", "....."]),
        vec![(4, 0)],
        "a short arm is not a box"
    );
    assert!(
        corner(&["ii.", "i..", "..."]).is_empty(),
        "three cells are a cluster, not a line"
    );
    assert!(
        corner(&["..iii", "iii..", "ii...", "i...."]).is_empty(),
        "the stepped edge of a shape is not a line"
    );
    assert!(
        corner(&["cbbbb", "bccbb", "bbcbb", "bbbcb"]).is_empty(),
        "a light L beside a darker shade is shading, not a line"
    );
    assert_eq!(corner(&["ibbbb", "biibb", "bbibb", "bbbib"]), vec![(2, 1)]);
}

#[test]
fn a_short_run_between_longer_ones_is_a_jagged_line() -> Result<(), String> {
    let jag = |r: &[&str]| cells_of(&critique(&rows(r), &[]), "jagged_line");
    let i = one(
        &critique(
            &rows(&[
                "iiii.........",
                "....i........",
                ".....iiii....",
                ".............",
            ]),
            &[],
        ),
        "jagged_line",
    )?;
    assert_eq!(i.cells, vec![(4, 1)]);
    assert!(i.message.contains("1 between 4 and 4"), "{}", i.message);
    assert_eq!(i.severity, Severity::Note);
    assert!(
        jag(&["iii......", "...iii...", "......iii"]).is_empty(),
        "even runs are a straight line"
    );
    assert!(
        jag(&[
            "i........",
            ".i.......",
            "..ii.....",
            "....iiii.",
            "........i"
        ])
        .is_empty(),
        "runs that grow along a curve are fine"
    );
    assert_eq!(
        jag(&[
            "i..", "i..", "i..", "i..", ".i.", "..i", "..i", "..i", "..i"
        ]),
        vec![(1, 4)],
        "the same down a column"
    );
    assert!(
        jag(&["iiii.iiii", "....i...."]).is_empty(),
        "a notch that comes back is not a step"
    );
    Ok(())
}

#[test]
fn a_one_cell_band_between_a_darker_and_a_lighter_colour_is_banding() {
    let band = |r: &[&str]| cells_of(&critique(&rows(r), &[]), "banding");
    assert_eq!(
        band(&["aaaaaa", "bbbbbb", "cccccc"]),
        (0..6).map(|x| (x, 1)).collect::<Vec<_>>()
    );
    assert!(
        band(&["aaaaaa", "bbbbbb", "bbbbbb", "cccccc"]).is_empty(),
        "two cells wide is a shape"
    );
    assert!(
        band(&["cccccc", "iiiiii", "cccccc"]).is_empty(),
        "a line through one colour is a detail"
    );
    assert!(
        band(&["bbbbbb", "iiiiii", "cccccc"]).is_empty(),
        "darker than both sides is a line"
    );
    assert!(
        band(&["aaa", "bbb", "ccc"]).is_empty(),
        "shorter than four cells"
    );
    assert!(
        band(&["......", "bbbbbb", "cccccc"]).is_empty(),
        "an edge next to nothing is not a band"
    );
    assert_eq!(
        band(&["abc", "abc", "abc", "abc"]),
        (0..4).map(|y| (1, y)).collect::<Vec<_>>()
    );
}
