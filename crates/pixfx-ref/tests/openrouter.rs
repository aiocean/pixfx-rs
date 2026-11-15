use pixfx_ref::base64::{decode, encode};
use pixfx_ref::openrouter::{Request, Tier, body, data_url, parse_answer};

#[test]
fn base64_matches_the_rfc_vectors_and_round_trips() -> Result<(), String> {
    for (raw, b64) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("foobar", "Zm9vYmFy")] {
        assert_eq!(encode(raw.as_bytes()), b64);
        assert_eq!(decode(b64)?, raw.as_bytes());
    }
    let bytes: Vec<u8> = (0..=255).collect();
    assert_eq!(decode(&encode(&bytes))?, bytes);
    assert!(decode("Zm9v!").is_err());
    Ok(())
}

#[test]
fn the_body_carries_what_was_set_and_leaves_out_the_rest() {
    let r = Request {
        model: Tier::Draft.model().into(),
        prompt: "draw".into(),
        aspect_ratio: Some("3:4".into()),
        background: Some("transparent".into()),
        quality: Tier::Draft.quality().map(String::from),
        references: vec![data_url("image/png", &[1, 2, 3])],
    };
    let b = body(&r);
    assert_eq!(b["model"], "openai/gpt-image-2.5-sunburst");
    assert_eq!(b["n"], 1);
    assert_eq!(b["background"], "transparent");
    assert_eq!(b["quality"], "auto");
    assert_eq!(b["input_references"][0]["image_url"]["url"], "data:image/png;base64,AQID");
    let bare = body(&Request { model: "m".into(), prompt: "p".into(), ..Request::default() });
    assert!(bare.get("aspect_ratio").is_none() && bare.get("input_references").is_none());
}

#[test]
fn an_answer_gives_its_images_and_cost_and_an_error_gives_the_apis_message() -> Result<(), String> {
    let png = [0x89, 0x50, 0x4e, 0x47, 0, 0];
    let ok = format!(r#"{{"data":[{{"b64_json":"{}"}}],"usage":{{"cost":0.065}}}}"#, encode(&png));
    let a = parse_answer(200, &ok)?;
    assert_eq!(a.images[0].bytes, png);
    assert_eq!((a.images[0].media_type.as_str(), a.images[0].extension), ("image/png", "png"));
    assert_eq!(a.cost_usd, Some(0.065));
    let err = parse_answer(402, r#"{"error":{"message":"Insufficient credits"}}"#);
    assert_eq!(err, Err("OpenRouter 402: Insufficient credits".into()));
    assert!(parse_answer(502, "<html>").is_err_and(|e| e.contains("non-JSON")));
    assert!(parse_answer(200, r#"{"data":[]}"#).is_err_and(|e| e.contains("no image")));
    Ok(())
}
