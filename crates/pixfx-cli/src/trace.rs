//! `pixfx trace`: the Rust table a traced pose is written as, for `pixfx-fighter`'s `traced/` folder.

use pixfx_fighter::traced::Traced;

/// The source of a table of `rects` (a Rust expression `&[Traced { .. }, ..]`), headed with how it was made.
pub fn table_source(rects: &[Traced], made_by: &str) -> String {
    let mut out = format!(
        "// Written by `{made_by}`.\n// Trace the art again rather than editing this by hand.\n&[\n"
    );
    for r in rects {
        out.push_str(&format!(
            "    Traced {{ dx: {}, dy: {}, w: {}, h: {}, role: Role::{:?} }},\n",
            r.dx, r.dy, r.w, r.h, r.role
        ));
    }
    out.push_str("]\n");
    out
}
