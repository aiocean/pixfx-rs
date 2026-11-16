use pixfx_ref::Spec;

const MIN: &str = r#"{"id":"shield/x","name":"X","look":"a shield.","subject":"shield",
  "rows":[{"name":"INIT","caption":"forms","text":"it forms"},{"name":"HIT RIGHT","caption":"","text":"it is hit","frames":2}]}"#;

#[test]
fn defaults_fill_the_grid_and_rows_keep_their_own_frame_count() -> Result<(), String> {
    let s = Spec::parse(MIN)?;
    assert_eq!((s.frames, s.cell, s.gap, s.scale), (3, 96, 14, 3));
    assert_eq!(s.frames_of(&s.rows[0]), 3);
    assert_eq!(s.frames_of(&s.rows[1]), 2);
    assert_eq!(s.cols(), 3);
    assert_eq!(s.slug(), "shield_x");
    Ok(())
}

#[test]
fn a_bad_spec_is_refused_with_its_reason() {
    let more_notes = MIN.replace(
        r#""text":"it forms""#,
        r#""text":"it forms","notes":["a","b","c","d"]"#,
    );
    assert!(Spec::parse(&more_notes).is_err_and(|e| e.contains("more notes than frames")));
    let one_col = MIN
        .replace(r#""frames":2"#, r#""frames":1"#)
        .replace(r#""text":"it forms""#, r#""text":"it forms","frames":1"#);
    assert!(Spec::parse(&one_col).is_err_and(|e| e.contains("at least 2 frames")));
    let no_rows = r#"{"id":"a","name":"A","look":"l","subject":"s","rows":[]}"#;
    assert!(Spec::parse(no_rows).is_err_and(|e| e.contains("at least one row")));
}

#[test]
fn every_shown_spec_parses() -> Result<(), String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/specs");
    let mut n = 0;
    for sub in ["shield", "."] {
        for e in std::fs::read_dir(dir.join(sub))
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") {
                Spec::parse(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("{}: {e}", p.display()))?;
                n += 1;
            }
        }
    }
    assert!(
        n >= 12,
        "expected the 11 shields and the template, found {n}"
    );
    Ok(())
}

#[test]
fn parts_widen_the_sheet_and_need_a_name_and_a_text() -> Result<(), String> {
    let with = MIN.replace(r#""subject":"shield","#, r#""subject":"shield","parts":[{"name":"A","text":"a"},{"name":"B","text":"b"},{"name":"C","text":"c"},{"name":"D","text":"d"}],"#);
    let s = Spec::parse(&with)?;
    assert_eq!(s.cols(), 4, "the parts row is the longest");
    let rows: Vec<(String, u32)> = s.sheet_rows();
    assert_eq!(
        rows,
        [
            ("PARTS".to_string(), 4),
            ("INIT".to_string(), 3),
            ("HIT RIGHT".to_string(), 2)
        ]
    );
    let blank = MIN.replace(
        r#""subject":"shield","#,
        r#""subject":"shield","parts":[{"name":"A","text":" "}],"#,
    );
    assert!(Spec::parse(&blank).is_err_and(|e| e.contains("needs a name and a text")));
    let named = MIN.replace(r#""name":"INIT""#, r#""name":"PARTS""#);
    assert!(Spec::parse(&named).is_err_and(|e| e.contains("parts row's name")));
    Ok(())
}

#[test]
fn forms_get_a_row_and_layers_split_each_moment_into_sheets() -> Result<(), String> {
    let with = MIN.replace(
        r#""subject":"shield","#,
        r#""subject":"shield","parts":[{"name":"MAIN","text":"m"},{"name":"PETAL","text":"p","pivot":"its base","forms":["up","curled","flat","edge-on"]}],"layers":[{"name":"SOLID","text":"s"},{"name":"LIGHT","text":"l"}],"#,
    );
    let s = Spec::parse(&with)?;
    assert_eq!(
        s.part_rows(),
        [("PARTS".to_string(), 2), ("PARTS-PETAL".to_string(), 4)]
    );
    assert_eq!(s.cols(), 4, "the forms row is the longest");
    let parts = s.parts_sheet();
    assert!(parts.rows.is_empty(), "the parts sheet holds no moment");
    assert_eq!(parts.sheet_rows().len(), 2);
    let hit = s.moment("hit right")?;
    assert!(hit.parts.is_empty(), "a moment's sheet draws no parts");
    assert_eq!(
        hit.drawn_parts.len(),
        2,
        "it names the parts drawn on the parts sheet"
    );
    assert_eq!(
        hit.sheet_rows(),
        [
            ("HIT RIGHT-SOLID".to_string(), 2),
            ("HIT RIGHT-LIGHT".to_string(), 2)
        ]
    );
    assert!(
        s.moment("BREAK")
            .is_err_and(|e| e.contains("no moment BREAK"))
    );
    let blank = with.replace(r#""flat""#, r#"" ""#);
    assert!(Spec::parse(&blank).is_err_and(|e| e.contains("blank form")));
    Ok(())
}
