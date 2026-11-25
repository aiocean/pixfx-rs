use pixfx_pixel::reference::Spec;
use pixfx_pixel::reference::prompt::prompt;

fn spec(name: &str) -> Result<Spec, String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    Spec::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)
}

#[test]
fn the_golden_bell_prompt_is_the_one_its_approved_sheet_was_drawn_from() -> Result<(), String> {
    let want = include_str!("golden-bell.prompt.txt").trim_end();
    assert_eq!(prompt(&spec("golden-bell.spec.json")?), want);
    Ok(())
}

#[test]
fn frame_notes_rules_and_a_row_frame_count_reach_the_prompt() -> Result<(), String> {
    let mut s = spec("golden-bell.spec.json")?;
    s.rows[0].frames = Some(5);
    s.rows[0].notes = vec![
        "only a few faint specks, under 20 pixels.".into(),
        String::new(),
        "half the bell".into(),
    ];
    s.rules = vec!["The bell grows gradually.".into()];
    let p = prompt(&s);
    assert!(p.contains("- INIT row (5 frames): the bell descends"));
    assert!(p.contains("\n  - frame 1: only a few faint specks, under 20 pixels.\n  - frame 3: half the bell.\n- IDLE row:"));
    assert!(!p.contains("frame 2:"), "an empty note says nothing");
    assert!(p.contains("then settling.\n- BREAK row: the bell fades from the rim upward and dissolves into rising golden motes.\n\nThe bell grows gradually.\n\nKeep the cell layout"));
    Ok(())
}

#[test]
fn a_grid_of_96_squares_draws_the_ruler_engine_pixel_for_engine_pixel() -> Result<(), String> {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/specs/shield/lotus.json");
    let mut s = Spec::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?;
    s.cell = 96;
    let p = prompt(&s);
    assert!(p.contains("plays on is 63 squares tall on this grid: its head is 11 squares wide, its hand 3 squares and its eye 2 squares."));
    Ok(())
}

#[test]
fn a_grid_and_parts_ask_for_whole_squares_and_each_part_alone() -> Result<(), String> {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/specs/shield/lotus.json");
    let s = Spec::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?;
    let p = prompt(&s);
    assert!(p.contains("Every cell is a grid of 64 by 64 squares"));
    assert!(p.contains("fill whole squares with one flat colour each"));
    // the duel's ruler at 64 squares for 96 engine pixels: the 63-pixel fighter is 42 squares, its 11-pixel head 7
    assert!(p.contains("The character this effect plays on is 42 squares tall on this grid: its head is 7 squares wide, its hand 2 squares and its eye 1 square."));
    assert!(p.contains("a sword as long as 17 squares"));
    assert!(p.contains("- PARTS 1, MAIN: the closed bud alone, every petal in place, with no glow, no specks and no pad; mark the centre of its base with one pure green (#00ff00) square.\n- PARTS 2, PETAL:"));
    assert!(p.contains("mark its base with one pure green (#00ff00) square, in every form."));
    assert!(p.contains("- the PETAL row: the same petal in 6 forms"));
    assert!(p.contains("  - PETAL 5: tumbling as it drifts away, seen edge-on, thin and bent."));
    assert!(p.contains("The other rows are the moments"));
    assert!(p.contains("built from the parts in the top rows"));
    assert!(p.contains("- INIT row: petals rise"));
    assert!(p.contains("Keep the cells, the grid size and the labels unchanged"));
    assert!(
        !p.contains("ordered dithering"),
        "flat squares, not dithering"
    );
    Ok(())
}

#[test]
fn a_moment_sheet_is_built_from_the_parts_sheet_layer_by_layer() -> Result<(), String> {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/specs/shield/lotus.json");
    let s = Spec::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?;
    let hit = prompt(&s.moment("HIT")?);
    assert!(!hit.contains("The top row, PARTS"), "no parts drawn on it");
    assert!(hit.contains(
        "The third image is this effect's parts sheet: MAIN, PETAL (in 6 forms), PAD, FX, LIGHT"
    ));
    assert!(hit.contains("never invent a new shape"));
    assert!(hit.contains("keeps MAIN's shape, number of pieces and size"));
    assert!(hit.contains("belongs only in a row that says the effect is struck or ends"));
    assert!(hit.contains("Each moment is drawn in 3 rows (HIT-SOLID, HIT-FX, HIT-LIGHT)"));
    let parts = prompt(&s.parts_sheet());
    assert!(parts.contains("- the PETAL row"));
    assert!(
        !parts.contains("stick figure"),
        "the parts sheet has no moments"
    );
    Ok(())
}
