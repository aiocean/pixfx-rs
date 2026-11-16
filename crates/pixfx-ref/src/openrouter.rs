//! The image model behind the references: OpenRouter's Image API
//! (<https://openrouter.ai/docs/features/multimodal/image-generation.md>, `POST /api/v1/images`). This module builds
//! the request body and reads the answer; sending it is the `generate` script's job.

use serde::Deserialize;
use serde_json::{Map, Value, json};

/// Where the request goes.
pub const ENDPOINT: &str = "https://openrouter.ai/api/v1/images";

/// The environment variable that holds the API key.
pub const KEY_VAR: &str = "OPENROUTER_API_KEY";

/// Quality tiers. Model ids listed by `GET /api/v1/images/models` on 2026-11-01. A draft sheet (two references,
/// 3:4) cost about $0.03 on a dark background and $0.065 on a transparent one on 2026-11-14.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Cheap and good enough for a reference: what the shields used.
    Draft,
    /// Same model at high quality.
    Standard,
    /// Gemini; ignores `background`, so its sheets come back opaque.
    Hero,
}

impl Tier {
    /// A tier by name: `draft`, `standard`, `hero`.
    pub fn by_name(name: &str) -> Option<Tier> {
        match name {
            "draft" => Some(Tier::Draft),
            "standard" => Some(Tier::Standard),
            "hero" => Some(Tier::Hero),
            _ => None,
        }
    }

    /// The model it calls.
    pub fn model(self) -> &'static str {
        match self {
            Tier::Draft | Tier::Standard => "openai/gpt-image-2.5-sunburst",
            Tier::Hero => "google/gemini-3-pro-image",
        }
    }

    /// The quality it asks for, when the model takes one.
    pub fn quality(self) -> Option<&'static str> {
        match self {
            Tier::Draft => Some("auto"),
            Tier::Standard => Some("high"),
            Tier::Hero => None,
        }
    }
}

/// One image request.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Request {
    pub model: String,
    pub prompt: String,
    /// `3:4`, `1:1`, ...
    pub aspect_ratio: Option<String>,
    /// `transparent` gives the picture an alpha channel (models that support it: gpt-image does, Gemini does not).
    pub background: Option<String>,
    pub quality: Option<String>,
    /// Reference images as data URLs ([`data_url`]) or http(s) URLs, in the order the prompt names them.
    pub references: Vec<String>,
}

/// The JSON body for `r`; fields left `None` are left out.
pub fn body(r: &Request) -> Value {
    let mut b = Map::new();
    b.insert("model".into(), json!(r.model));
    b.insert("prompt".into(), json!(r.prompt));
    b.insert("n".into(), json!(1));
    let mut opt = |k: &str, v: &Option<String>| {
        if let Some(v) = v {
            b.insert(k.into(), json!(v));
        }
    };
    opt("aspect_ratio", &r.aspect_ratio);
    opt("background", &r.background);
    opt("quality", &r.quality);
    if !r.references.is_empty() {
        let refs: Vec<Value> = r
            .references
            .iter()
            .map(|url| json!({"type": "image_url", "image_url": {"url": url}}))
            .collect();
        b.insert("input_references".into(), Value::Array(refs));
    }
    Value::Object(b)
}

/// A file's media type from its extension (`png`, `jpg`, `jpeg`, `webp`), the only kinds a reference may be.
pub fn media_type(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// `bytes` as a data URL of `media_type`.
pub fn data_url(media_type: &str, bytes: &[u8]) -> String {
    format!("data:{media_type};base64,{}", crate::base64::encode(bytes))
}

/// An image the model sent back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub media_type: String,
    /// `png`, `jpg`, `webp`.
    pub extension: &'static str,
}

/// What a request gave: the images and what they cost, when the API said.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub images: Vec<Image>,
    pub cost_usd: Option<f64>,
}

#[derive(Deserialize)]
struct Response {
    data: Vec<Item>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Item {
    b64_json: String,
    media_type: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    cost: Option<f64>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorMessage,
}

#[derive(Deserialize)]
struct ErrorMessage {
    message: String,
}

fn extension(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

/// The API leaves `media_type` out only when it could not tell the format; the magic bytes tell it then.
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..10) == Some(b"WE") {
        Some("image/webp")
    } else {
        None
    }
}

/// The answer to a request from its HTTP status and body. A status outside 200..300 fails with the API's own
/// message; so does an answer with no image or an image in a format other than png, jpeg or webp.
pub fn parse_answer(status: u16, text: &str) -> Result<Answer, String> {
    let json: Value = serde_json::from_str(text)
        .map_err(|_| format!("OpenRouter {status}: non-JSON response"))?;
    if !(200..300).contains(&status) {
        let msg = serde_json::from_value::<ErrorBody>(json)
            .map(|e| e.error.message)
            .unwrap_or_else(|_| "no error message".into());
        return Err(format!("OpenRouter {status}: {msg}"));
    }
    let r: Response =
        serde_json::from_value(json).map_err(|e| format!("OpenRouter answer: {e}"))?;
    if r.data.is_empty() {
        return Err("OpenRouter answer: no image".into());
    }
    let images = r
        .data
        .into_iter()
        .map(|item| {
            let bytes = crate::base64::decode(&item.b64_json)?;
            let media = item
                .media_type
                .clone()
                .or_else(|| sniff(&bytes).map(String::from))
                .ok_or("unrecognized image format (media_type absent)")?;
            let ext =
                extension(&media).ok_or_else(|| format!("unrecognized image format ({media})"))?;
            Ok(Image {
                bytes,
                media_type: media,
                extension: ext,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Answer {
        images,
        cost_usd: r.usage.and_then(|u| u.cost),
    })
}
