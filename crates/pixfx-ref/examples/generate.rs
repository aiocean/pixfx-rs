//! Step 2: ask the image model to draw the template. This spends money (about $0.065 a draft sheet).
//!
//!   cargo run -p pixfx-ref --example generate -- <dir> [--tier draft|standard|hero] [--aspect W:H] [--dry-run]
//!
//! `<dir>` is what `template` wrote (`prompt.txt`, `template.png`, `style.png`). Sends the prompt with the two
//! images, in that order, asking for a transparent background, and writes the answer into the next free
//! `<dir>/draw-<n>/`: `image-01.png` and `job.json` (model, request settings, cost). Every call is appended to
//! `target/ref/spend.jsonl`. Nothing is overwritten: a rerun draws again and costs again. The aspect ratio is the
//! one of [`ASPECTS`] nearest the template's shape, unless `--aspect` says otherwise.
//!
//! The API key is read from `OPENROUTER_API_KEY` and handed to `curl` on its standard input, so it never shows in
//! the process list. `--dry-run` prints the request without the images and spends nothing.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use common::{Args, exit, read_png, read_text, write_text};
use pixfx_ref::openrouter::{ENDPOINT, KEY_VAR, Request, Tier, body, data_url, parse_answer};
use serde_json::json;

const USAGE: &str = "usage: generate <dir> [--tier draft|standard|hero] [--aspect W:H] [--dry-run]";

/// Aspect ratios the image API takes.
const ASPECTS: [(u32, u32); 7] = [(1, 1), (3, 4), (4, 3), (2, 3), (3, 2), (9, 16), (16, 9)];

/// The ratio of [`ASPECTS`] nearest `w` x `h`, as `W:H`.
fn nearest_aspect(w: u32, h: u32) -> String {
    let r = f64::from(w.max(1)) / f64::from(h.max(1));
    let off = |&(a, b): &(u32, u32)| {
        let t = f64::from(a) / f64::from(b);
        (r / t).max(t / r)
    };
    let (a, b) = ASPECTS.iter().copied().min_by(|x, y| off(x).total_cmp(&off(y))).unwrap_or((1, 1));
    format!("{a}:{b}")
}

fn main() -> std::process::ExitCode {
    exit("generate", run())
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let tier_name = args.value("--tier")?.unwrap_or_else(|| "draft".into());
    let tier = Tier::by_name(&tier_name).ok_or_else(|| format!("--tier {tier_name}: draft, standard or hero"))?;
    let aspect = args.value("--aspect")?;
    let dry = args.flag("--dry-run");
    let dir = PathBuf::from(&args.rest(1..=1)?[0]);
    let aspect = match aspect {
        Some(a) => a,
        None => {
            let t = read_png(&dir.join("template.png"))?;
            nearest_aspect(t.w, t.h)
        }
    };
    let image = |name: &str| -> Result<String, String> {
        let p = dir.join(name);
        let bytes = std::fs::read(&p).map_err(|e| format!("reading {}: {e} (run the template script first)", p.display()))?;
        Ok(data_url("image/png", &bytes))
    };
    let mut req = Request {
        model: tier.model().into(),
        prompt: read_text(&dir.join("prompt.txt"))?.trim().to_string(),
        aspect_ratio: Some(aspect),
        background: Some("transparent".into()),
        quality: tier.quality().map(String::from),
        references: vec![image("template.png")?, image("style.png")?],
    };
    if dry {
        let n = req.references.len();
        req.references.clear();
        let mut b = body(&req);
        b["input_references"] = json!(format!("{n} reference image(s)"));
        println!("{}", serde_json::to_string_pretty(&b).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let key = std::env::var(KEY_VAR).map_err(|_| format!("{KEY_VAR} is not set"))?;
    let out = next_free(&dir)?;
    std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    let body_file = out.join("request.json");
    write_text(&body_file, &body(&req).to_string())?;
    let sent = send(&key, &body_file);
    std::fs::remove_file(&body_file).map_err(|e| format!("removing {}: {e}", body_file.display()))?;
    let (status, text) = sent?;
    let answer = parse_answer(status, &text)?;
    let mut files = Vec::new();
    for (i, img) in answer.images.iter().enumerate() {
        let f = out.join(format!("image-{:02}.{}", i + 1, img.extension));
        std::fs::write(&f, &img.bytes).map_err(|e| format!("writing {}: {e}", f.display()))?;
        files.push(f.display().to_string());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let job = json!({
        "created_unix": now, "route": "openrouter", "model": req.model, "tier": tier_name,
        "aspect_ratio": req.aspect_ratio, "background": req.background, "quality": req.quality,
        "files": files, "cost_usd": answer.cost_usd,
    });
    write_text(&out.join("job.json"), &format!("{job:#}\n"))?;
    let ledger = Path::new("target/ref/spend.jsonl");
    let line = json!({"created_unix": now, "model": req.model, "cost_usd": answer.cost_usd, "out": out.display().to_string()});
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger)
        .map_err(|e| format!("opening {}: {e}", ledger.display()))?;
    writeln!(f, "{line}").map_err(|e| format!("writing {}: {e}", ledger.display()))?;
    println!("{}", json!({"files": files, "cost_usd": answer.cost_usd}));
    Ok(())
}

/// `<dir>/draw-<n>` for the first `n` from 1 not taken yet.
fn next_free(dir: &Path) -> Result<PathBuf, String> {
    (1..10_000)
        .map(|n| dir.join(format!("draw-{n}")))
        .find(|p| !p.exists())
        .ok_or_else(|| format!("{}: no free draw-<n> left", dir.display()))
}

/// POST the body in `body_file` with curl; the key goes in on stdin as a curl config line. Gives the HTTP status
/// and the body of the answer.
fn send(key: &str, body_file: &Path) -> Result<(u16, String), String> {
    let mut child = Command::new("curl")
        .args(["-sS", "--max-time", "300", "-X", "POST", ENDPOINT, "-H", "Content-Type: application/json", "-K", "-"])
        .arg("--data-binary")
        .arg(format!("@{}", body_file.display()))
        .args(["-w", "\n%{http_code}"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("running curl: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("curl: no stdin")?;
    writeln!(stdin, "header = \"Authorization: Bearer {key}\"").map_err(|e| format!("curl stdin: {e}"))?;
    drop(stdin);
    let out = child.wait_with_output().map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').ok_or("curl: no status line")?;
    let status = code.trim().parse().map_err(|_| format!("curl: bad status {code}"))?;
    Ok((status, body.to_string()))
}
