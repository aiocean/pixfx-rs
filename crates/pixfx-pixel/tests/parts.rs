// Parts: an agent's spec splits a sprite into named parts, each painted cell in exactly one, with a pivot where a
// part meets its parent.
use pixfx_pixel::Map;
use pixfx_pixel::parts::{
    Component, PartSpec, Rule, SplitError, components, parse_spec, spec_text, split,
};

/// A 8x10 figure: head (colour 0) rows 0..3 cols 2..6, body (1) rows 3..8 cols 1..7, arm (2) col 7 rows 4..7.
/// Plus a lone speck (3) at (0, 9).
fn figure() -> Map {
    let mut m = Map::empty(8, 10);
    let mut put = |x: u32, y: u32, c: usize| {
        if let Some(cell) = m.cells.get_mut((y * 8 + x) as usize) {
            *cell = Some(c);
        }
    };
    for y in 0..3 {
        for x in 2..6 {
            put(x, y, 0);
        }
    }
    for y in 3..8 {
        for x in 1..7 {
            put(x, y, 1);
        }
    }
    for y in 4..7 {
        put(7, y, 2);
    }
    put(0, 9, 3);
    m
}

fn part(name: &str, parent: Option<&str>, rule: Rule) -> PartSpec {
    PartSpec {
        name: name.to_string(),
        parent: parent.map(str::to_string),
        rule,
        joint: None,
    }
}

fn boxes() -> Vec<PartSpec> {
    vec![
        part("head", Some("body"), Rule::Box([0, 0, 8, 3])),
        part("arm", Some("body"), Rule::Box([7, 0, 1, 10])),
        part("body", None, Rule::Box([0, 3, 7, 5])),
        part("speck", None, Rule::Rest),
    ]
}

#[test]
fn components_are_8_connected_and_ordered_by_first_cell() {
    let mut m = figure();
    // A cell touching the body only at a corner joins it.
    if let Some(c) = m.cells.get_mut(8 * 8 + 7) {
        *c = Some(3);
    }
    let got = components(&m);
    assert_eq!(
        got,
        vec![
            Component {
                cells: 12 + 30 + 3 + 1,
                bbox: [1, 0, 7, 9],
                colors: vec![0, 1, 2, 3]
            },
            Component {
                cells: 1,
                bbox: [0, 9, 1, 1],
                colors: vec![3]
            },
        ]
    );
}

#[test]
fn every_painted_cell_lands_in_exactly_one_part_first_rule_wins() -> Result<(), SplitError> {
    let m = figure();
    let parts = split(&m, &boxes())?;
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["head", "arm", "body", "speck"]);
    for (i, cell) in m.cells.iter().enumerate() {
        let holders: Vec<&str> = parts
            .iter()
            .filter(|p| p.map.cells.get(i).copied().flatten().is_some())
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(
            holders.len(),
            usize::from(cell.is_some()),
            "cell {i}: {holders:?}"
        );
        for p in &parts {
            if let Some(c) = p.map.cells.get(i).copied().flatten() {
                assert_eq!(Some(c), *cell, "part {} keeps the colour", p.name);
            }
        }
    }
    let bbox = |n: &str| parts.iter().find(|p| p.name == n).map(|p| p.bbox);
    assert_eq!(bbox("head"), Some([2, 0, 4, 3]));
    assert_eq!(bbox("arm"), Some([7, 4, 1, 3]));
    assert_eq!(bbox("body"), Some([1, 3, 6, 5]));
    assert_eq!(bbox("speck"), Some([0, 9, 1, 1]));
    // The arm box is listed before the body box, so the overlapping column 7 goes to the arm.
    let first_wins = vec![
        part("arm", None, Rule::Box([6, 0, 2, 10])),
        part("rest", None, Rule::Rest),
    ];
    let parts = split(&m, &first_wins)?;
    assert_eq!(parts.first().map(|p| p.bbox), Some([6, 3, 2, 5]));
    Ok(())
}

#[test]
fn pivot_is_the_middle_of_the_border_with_the_parent() -> Result<(), SplitError> {
    let parts = split(&figure(), &boxes())?;
    let pivot = |n: &str| parts.iter().find(|p| p.name == n).and_then(|p| p.pivot);
    // Head cols 2..6 sit on the body along y = 3: the middle is x = 4.
    assert_eq!(pivot("head"), Some((4.0, 3.0)));
    // Arm rows 4..7 hang off the body's right side along x = 7: the middle is y = 5.5.
    assert_eq!(pivot("arm"), Some((7.0, 5.5)));
    assert_eq!(pivot("body"), None);
    Ok(())
}

#[test]
fn colors_rule_takes_only_its_colours_inside_its_box() -> Result<(), SplitError> {
    let spec = vec![
        part(
            "upper",
            None,
            Rule::Colors {
                colors: vec![1],
                within: Some([0, 0, 8, 5]),
            },
        ),
        part("rest", Some("upper"), Rule::Rest),
    ];
    let parts = split(&figure(), &spec)?;
    assert_eq!(parts.first().map(|p| p.bbox), Some([1, 3, 6, 2]));
    assert_eq!(
        parts.first().map(|p| p.map.cells.iter().flatten().count()),
        Some(12)
    );
    Ok(())
}

#[test]
fn an_unknown_parent_is_refused_naming_both() {
    let mut spec = boxes();
    if let Some(s) = spec.first_mut() {
        s.parent = Some("torso".to_string());
    }
    let msg = split(&figure(), &spec)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(msg.contains("head") && msg.contains("torso"), "{msg}");
}

#[test]
fn a_parent_that_does_not_touch_is_refused_naming_both() {
    let mut spec = boxes();
    if let Some(s) = spec.first_mut() {
        s.parent = Some("speck".to_string());
    }
    let msg = split(&figure(), &spec)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        msg.contains("head") && msg.contains("speck") && msg.contains("touch"),
        "{msg}"
    );
}

#[test]
fn cells_no_rule_takes_without_rest_are_refused_with_their_count() {
    let mut spec = boxes();
    spec.pop();
    assert_eq!(
        split(&figure(), &spec).err(),
        Some(SplitError(
            "1 painted cells match no rule and the spec has no rest part".to_string()
        ))
    );
}

#[test]
fn split_is_deterministic() {
    let a = split(&figure(), &boxes());
    let b = split(&figure(), &boxes());
    assert_eq!(a, b);
}

#[test]
fn parse_spec_round_trips_the_three_rule_forms() {
    let spec = vec![
        part("head", Some("body"), Rule::Box([2, 0, 4, 3])),
        part(
            "cape",
            Some("body"),
            Rule::Colors {
                colors: vec![0, 27, 61],
                within: None,
            },
        ),
        part(
            "belt",
            Some("body"),
            Rule::Colors {
                colors: vec![3],
                within: Some([1, 6, 6, 1]),
            },
        ),
        part("body", None, Rule::Rest),
    ];
    let text = spec_text(&spec);
    assert_eq!(
        text,
        "head body box 2,0,4,3\ncape body colors a,B,9\nbelt body colors d in 1,6,6,1\nbody - rest\n"
    );
    assert_eq!(parse_spec(&text), Ok(spec));
    let commented = format!("# parts of the hero\n\n{text}");
    assert_eq!(parse_spec(&commented).map(|s| s.len()), Ok(4));
}

#[test]
fn parse_spec_names_the_bad_line() {
    let msg = parse_spec("head - box 1,2,3\n")
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(msg.starts_with("line 1:"), "{msg}");
    let msg = parse_spec("a - rest\nhead - colors !\n")
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(msg.starts_with("line 2:") && msg.contains('!'), "{msg}");
}

#[test]
fn a_poly_takes_the_cells_inside_its_outline_and_at_names_the_joint() -> Result<(), SplitError> {
    // An arm lying diagonally over a body: the triangle (4,0) (8,0) (8,4) takes cells above the diagonal only.
    let mut m = Map::empty(8, 8);
    for c in m.cells.iter_mut() {
        *c = Some(0);
    }
    let spec = parse_spec("arm body poly 4,0 8,0 8,4 at 6,1\nbody - rest\n")?;
    let parts = split(&m, &spec)?;
    let arm = parts.first().ok_or(SplitError("no arm".into()))?;
    let taken: Vec<(u32, u32)> = (0..8u32)
        .flat_map(|y| (0..8u32).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            arm.map
                .cells
                .get((y * 8 + x) as usize)
                .copied()
                .flatten()
                .is_some()
        })
        .collect();
    // Cell centres above or on the line from (4,0) to (8,4): y <= x - 4.
    let expect = vec![
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
        (5, 1),
        (6, 1),
        (7, 1),
        (6, 2),
        (7, 2),
        (7, 3),
    ];
    assert_eq!(taken, expect);
    // The same triangle with its diagonal on the other side of the shape takes the diagonal's cells too.
    let mirrored = parse_spec("arm body poly 0,0 4,0 0,4\nbody - rest\n")?;
    let left = split(&m, &mirrored)?;
    let on_diagonal = |x: u32, y: u32| {
        left.first()
            .and_then(|p| p.map.cells.get((y * 8 + x) as usize).copied().flatten())
    };
    assert!(
        on_diagonal(3, 0).is_some() && on_diagonal(0, 3).is_some() && on_diagonal(3, 1).is_none()
    );
    assert_eq!(arm.pivot, Some((6.0, 1.0)));
    assert_eq!(parse_spec(&spec_text(&spec))?, spec);
    assert!(parse_spec("arm body poly 1,1 2,2\n").is_err());
    Ok(())
}
