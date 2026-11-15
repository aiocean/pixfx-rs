use pixfx_ref::Spec;
use pixfx_ref::prompt::prompt;

fn spec(name: &str) -> Result<Spec, String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/specs/shield").join(name);
    Spec::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)
}

#[test]
fn the_golden_bell_prompt_is_the_one_its_approved_sheet_was_drawn_from() -> Result<(), String> {
    let want = include_str!("golden-bell.prompt.txt").trim_end();
    assert_eq!(prompt(&spec("golden-bell.json")?), want);
    Ok(())
}

#[test]
fn frame_notes_rules_and_a_row_frame_count_reach_the_prompt() -> Result<(), String> {
    let mut s = spec("golden-bell.json")?;
    s.rows[0].frames = Some(5);
    s.rows[0].notes = vec!["only a few faint specks, under 20 pixels.".into(), String::new(), "half the bell".into()];
    s.rules = vec!["The bell grows gradually.".into()];
    let p = prompt(&s);
    assert!(p.contains("- INIT row (5 frames): the bell descends"));
    assert!(p.contains("\n  - frame 1: only a few faint specks, under 20 pixels.\n  - frame 3: half the bell.\n- IDLE row:"));
    assert!(!p.contains("frame 2:"), "an empty note says nothing");
    assert!(p.contains("then settling.\n- BREAK row: the bell fades from the rim upward and dissolves into rising golden motes.\n\nThe bell grows gradually.\n\nKeep the cell layout"));
    Ok(())
}
