//! Crystal — session snapshot. Pin table + slot tensors → hash → loadable.
//!
//! "Crystal = snapshot. Field = flow. Commit between them deliberately."
//! — CCP v0.1 §9
//!
//! Crystal hash is content-addressed. Two sessions arriving at the same
//! pin/slot configuration hash to the same address and merge for free.

use crate::grid::PinGrid;
use crate::scope::ScopedGrid;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crystal {
    /// BLAKE3 of the full serialized content (pins + edges + turn)
    pub id: String,
    pub session_id: String,
    pub grid: PinGrid,
    pub timestamp: u64,
    pub tokens_saved: usize,
}

impl Crystal {
    pub fn seal(grid: &PinGrid, session_id: &str) -> Self {
        let tokens_saved = grid.total_saved();
        let content = serde_json::to_string(grid).unwrap_or_default();
        let id = format!("sha256:{}", blake3::hash(content.as_bytes()).to_hex());
        Crystal {
            id,
            session_id: session_id.to_string(),
            grid: grid.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            tokens_saved,
        }
    }
}

fn crystals_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".gently").join("crystals")
}

fn grid_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".gently").join("pins.json")
}

pub fn load_grid() -> PinGrid {
    let path = grid_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_else(|_| PinGrid::new())
    } else {
        PinGrid::new()
    }
}

pub fn save_grid(grid: &PinGrid) {
    let path = grid_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(grid) {
        let _ = std::fs::write(path, data);
    }
}

/// Seal a ScopedGrid — stores the chat-level grid as a resumable Crystal.
/// Also persists the delta to the project-level table (project pins only).
pub fn dump_scoped(sg: &ScopedGrid) -> Crystal {
    let crystal = Crystal::seal(&sg.chat, &sg.session_id);
    let dir = crystals_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", &crystal.id[7..23]));
    if let Ok(data) = serde_json::to_string_pretty(&crystal) {
        let _ = std::fs::write(path, data);
    }
    crystal
}

/// Seal the current grid as a Crystal, persist it, return the id.
pub fn dump(grid: &PinGrid, session_id: &str) -> Crystal {
    let crystal = Crystal::seal(grid, session_id);
    let dir = crystals_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", &crystal.id[7..23])); // first 16 hex chars
    if let Ok(data) = serde_json::to_string_pretty(&crystal) {
        let _ = std::fs::write(path, data);
    }
    crystal
}

/// Load a Crystal by id prefix (first N chars of hash hex).
pub fn load_crystal(id_prefix: &str) -> Option<Crystal> {
    let dir = crystals_dir();
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(id_prefix) {
            if let Ok(data) = std::fs::read_to_string(entry.path()) {
                return serde_json::from_str(&data).ok();
            }
        }
    }
    None
}

/// List all crystals, most recent first.
pub fn list_crystals() -> Vec<Crystal> {
    let dir = crystals_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return vec![] };
    let mut crystals: Vec<Crystal> = entries
        .flatten()
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            serde_json::from_str(&data).ok()
        })
        .collect();
    crystals.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    crystals
}
