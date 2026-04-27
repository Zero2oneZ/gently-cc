//! Session stats: ~/.gently/codie-session.json
//! Tracks tokens saved across turns for the ⚡ stats line.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub tokens_original: u64,
    pub tokens_compressed: u64,
    pub tokens_saved: u64,
    pub pct_sum: f64,
    pub turns: u64,
}

impl SessionStats {
    pub fn avg_pct(&self) -> f64 {
        if self.turns == 0 { 0.0 } else { self.pct_sum / self.turns as f64 }
    }

    pub fn record(&mut self, original: usize, compressed: usize, pct: f64) {
        self.tokens_original += original as u64;
        self.tokens_compressed += compressed as u64;
        let saved = original.saturating_sub(compressed) as u64;
        self.tokens_saved += saved;
        self.pct_sum += pct;
        self.turns += 1;
    }
}

fn session_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".gently").join("codie-session.json"))
}

pub fn load() -> SessionStats {
    let path = match session_path() { Some(p) => p, None => return Default::default() };
    let data = match std::fs::read_to_string(&path) { Ok(d) => d, Err(_) => return Default::default() };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(stats: &SessionStats) {
    let path = match session_path() { Some(p) => p, None => return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(stats) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn update_and_save(original: usize, compressed: usize, pct: f64) -> SessionStats {
    let mut stats = load();
    stats.record(original, compressed, pct);
    save(&stats);
    stats
}

/// Format the ⚡ stats line emitted to stderr.
pub fn stats_line(orig: usize, comp: usize, pct: f64, session: &SessionStats) -> String {
    format!(
        "⚡ CODIE {:.1}% · {}→{} tokens  [session: {} saved · {} turns · {:.1}% avg]",
        pct, orig, comp, session.tokens_saved, session.turns, session.avg_pct()
    )
}
