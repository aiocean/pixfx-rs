// Ramps: dark and cool to light and warm round their base; palettes read from .gpl and .hex files.
use pixfx_pixel::Rgba;
use pixfx_pixel::oklab::{lab, rgb};
use pixfx_pixel::ramp::ramp;
use pixfx_pixel::tool::Palette;

#[test]
fn oklab_goes_back_to_the_same_srgb_colour() {
    for c in [
        Rgba(139, 90, 43, 255),
        Rgba(0, 0, 0, 255),
        Rgba(255, 255, 255, 9),
        Rgba(51, 102, 153, 200),
    ] {
        assert_eq!(rgb(lab(c), c.3), c);
    }
}

#[test]
fn a_ramp_runs_from_dark_and_cool_to_light_and_warm_through_its_base() {
    let wood = Rgba(139, 90, 43, 255);
    let steps = ramp(wood, 5);
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[2], wood, "the middle step is the base colour");
    let labs: Vec<_> = steps.iter().map(|c| lab(*c)).collect();
    assert!(
        labs.windows(2).all(|w| w[1].l > w[0].l),
        "each step is lighter: {labs:?}"
    );
    assert!(
        labs[4].b > labs[0].b,
        "the light end is yellower than the dark end"
    );
    assert!(steps.iter().all(|c| c.3 == 255), "alpha is kept");
    assert_eq!(ramp(wood, 1).len(), 2, "a ramp has two steps at least");
    assert_eq!(ramp(wood, 99).len(), 16, "and sixteen at most");
}

#[test]
fn ramps_are_named_by_material_and_step_and_read_back_from_a_gpl() -> Result<(), String> {
    let p = Palette::ramps(
        &[
            ("skin".to_string(), Rgba(217, 154, 108, 255)),
            ("cloth".to_string(), Rgba(51, 85, 170, 255)),
        ],
        3,
    );
    assert_eq!(
        p.names,
        [
            "skin-1", "skin-2", "skin-3", "cloth-1", "cloth-2", "cloth-3"
        ]
    );
    assert_eq!(Palette::parse_file(&p.gpl("hero"))?, p);
    Ok(())
}

#[test]
fn hex_lists_and_gimp_palettes_are_read() -> Result<(), String> {
    let hex = Palette::parse_file("1a1c2c\n#5D275D\n\n; a comment\n")?;
    assert_eq!(
        hex.colors,
        [Rgba(0x1a, 0x1c, 0x2c, 255), Rgba(0x5d, 0x27, 0x5d, 255)]
    );
    assert_eq!(hex.names, ["", ""]);
    let gpl = Palette::parse_file(
        "GIMP Palette\nName: Sprite\nColumns: 4\n#\n 26  28  44\tnight sky\n255 205 117\n",
    )?;
    assert_eq!(
        gpl.colors,
        [Rgba(26, 28, 44, 255), Rgba(255, 205, 117, 255)]
    );
    assert_eq!(gpl.names, ["night sky", ""]);
    assert!(Palette::parse_file("not a colour").is_err());
    assert!(Palette::parse_file("GIMP Palette\n300 0 0\n").is_err());
    assert!(Palette::parse_file("\n\n").is_err());
    Ok(())
}
