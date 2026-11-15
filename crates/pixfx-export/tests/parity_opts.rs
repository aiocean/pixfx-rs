use pixfx_core::effect::OptValue;
use pixfx_export::parity::ParityOpts;

fn parse(args: &[&str]) -> Result<ParityOpts, String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    ParityOpts::parse(&owned)
}

#[test]
fn seed_flip_and_numbers_become_options() {
    let o = parse(&["seed=3", "flip=1", "hits=2", "from=-1"]).map(|c| c.options());
    assert_eq!(o.as_ref().map(|o| o.seed), Ok(Some(3)));
    assert_eq!(o.as_ref().map(|o| o.flip), Ok(true));
    assert_eq!(
        o.as_ref().map(|o| o.extra.get("hits").cloned()),
        Ok(Some(OptValue::Num(2.0)))
    );
    assert_eq!(
        o.as_ref().map(|o| o.extra.get("from").cloned()),
        Ok(Some(OptValue::Num(-1.0)))
    );
}

#[test]
fn json_for_the_reference_script_carries_the_same_options() {
    let json = parse(&["seed=3", "flip=1", "hits=2"]).map(|c| c.to_json());
    assert_eq!(
        json.as_deref(),
        Ok(r#"{"seed":3,"options":{"flip":true,"hits":2}}"#)
    );
}

#[test]
fn no_options_is_an_empty_object() {
    assert_eq!(
        parse(&[]).map(|c| c.to_json()).as_deref(),
        Ok(r#"{"options":{}}"#)
    );
}

#[test]
fn malformed_options_are_refused() {
    assert!(parse(&["hits"]).is_err());
    assert!(parse(&["hits=lots"]).is_err());
    assert!(parse(&["flip=2"]).is_err());
    assert!(parse(&["seed=-1"]).is_err());
    assert!(parse(&["bad key=1"]).is_err());
}
