use std::process::Command;

#[test]
fn parity_on_an_effect_not_ported_says_so() -> std::io::Result<()> {
    let out = Command::new(env!("CARGO_BIN_EXE_pixfx"))
        .args([
            "parity",
            "fire/no-such-effect",
            "--out",
            env!("CARGO_TARGET_TMPDIR"),
        ])
        .output()?;
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "pixfx: no pixfx effect fire/no-such-effect (not ported yet?)\n"
    );
    assert!(out.stdout.is_empty());
    Ok(())
}

#[test]
fn pixfx_without_a_command_shows_usage() -> std::io::Result<()> {
    let out = Command::new(env!("CARGO_BIN_EXE_pixfx")).output()?;
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage: pixfx parity <id>"));
    Ok(())
}
