//! Claude Code UserPromptSubmit hook mode.
//! Reads {"prompt": "..."} from stdin.
//! Writes {"continue": true, "prompt": "...", "unknowns": [...]} to stdout.
//! Stats line emitted to stderr.
//!
//! "unknowns" are tokens with no glyph/abbrev rule — generative pin candidates
//! for the codec layer. The JS pipeline seeds these into the foam automatically.

use crate::compress;
use crate::stats;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct HookInput {
    prompt: Option<String>,
}

#[derive(Serialize)]
struct HookOutput {
    r#continue: bool,
    prompt: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unknowns: Vec<String>,
}

pub fn run() {
    use std::io::Read;
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        let out = HookOutput { r#continue: true, prompt: String::new(), unknowns: vec![] };
        println!("{}", serde_json::to_string(&out).unwrap());
        return;
    }

    let parsed: HookInput = serde_json::from_str(input.trim()).unwrap_or(HookInput { prompt: None });
    let text = parsed.prompt.unwrap_or_default();

    if text.chars().count() < 60 {
        let out = HookOutput { r#continue: true, prompt: text, unknowns: vec![] };
        println!("{}", serde_json::to_string(&out).unwrap());
        return;
    }

    let c = compress::compress(&text);
    let pct = c.pct_saved();

    let prompt = if pct >= 10.0 { c.output.clone() } else { text };
    let unknowns = c.unknowns;

    let session = stats::update_and_save(c.original_tokens, c.compressed_tokens, pct);
    eprintln!("{}", stats::stats_line(c.original_tokens, c.compressed_tokens, pct, &session));

    let out = HookOutput { r#continue: true, prompt, unknowns };
    println!("{}", serde_json::to_string(&out).unwrap());
}
