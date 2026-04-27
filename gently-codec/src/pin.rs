use serde::{Deserialize, Serialize};

/// A named concept, addressed by BLAKE3 of its canonical label.
/// Defines once, referenced everywhere. Single token → entire referent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,           // "sha256:{hex}" — BLAKE3 of label
    pub name: String,         // short name, e.g. "auth"
    pub label: String,        // full definition text
    pub ref_count: u32,       // how many times referenced this session
    pub tombstone: bool,      // [*name: ~] — deprecated, address still valid
    pub created_turn: u32,
}

impl Pin {
    pub fn new(name: &str, label: &str, turn: u32) -> Self {
        let id = format!("sha256:{}", blake3::hash(label.as_bytes()).to_hex());
        Pin {
            id,
            name: name.to_string(),
            label: label.to_string(),
            ref_count: 0,
            tombstone: false,
            created_turn: turn,
        }
    }

    /// Compact reference token emitted on wire: *name
    pub fn ref_token(&self) -> String {
        format!("*{}", self.name)
    }

    /// Tokens saved per use = label_tokens - 1 (the *name token)
    pub fn tokens_saved_per_use(&self) -> usize {
        let label_tokens = (self.label.chars().count() / 4).max(1);
        label_tokens.saturating_sub(1)
    }

    /// Total tokens saved so far this session
    pub fn total_saved(&self) -> usize {
        self.ref_count as usize * self.tokens_saved_per_use()
    }
}
