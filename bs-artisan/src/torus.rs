use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

/// CCP agency frames — the dominant operator context a slot fires under.
/// Maps to CCP prefix operators: ? > < ! = ~ :
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum AgencyFrame {
    #[default]
    None    = 0,
    Query   = 1,  // ?x — in motion, non-destructive
    Command = 2,  // >x — caller frame, planted
    Return  = 3,  // <x — callee return
    Assert  = 4,  // !x — planted assertion
    Ground  = 5,  // =x — settled truth
    Fuzzy   = 6,  // ~x — exploratory
    Define  = 7,  // :x — establishes referent
}

impl AgencyFrame {
    pub fn from_u8(n: u8) -> Self {
        match n {
            1 => Self::Query, 2 => Self::Command, 3 => Self::Return,
            4 => Self::Assert, 5 => Self::Ground, 6 => Self::Fuzzy,
            7 => Self::Define, _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torus {
    pub id: [u8; 32],
    pub label: String,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub winding: u8,
    pub bs: f64,
    pub connections: Vec<[u8; 32]>,

    // CCP slot tensor fields
    pub count: u32,
    pub last_seen_turn: u32,
    pub cooccur: Vec<[u8; 32]>,     // slot ids that fire alongside this one
    pub agency_bias: AgencyFrame,   // dominant operator frame
}

impl Torus {
    pub fn new(label: &str, tokens: usize) -> Self {
        let id = blake3::hash(label.as_bytes()).into();
        Torus {
            id,
            label: label.to_string(),
            major_radius: 1.0,
            minor_radius: tokens_to_radius(tokens),
            winding: 1,
            bs: 0.5,
            connections: Vec::new(),
            count: 1,
            last_seen_turn: 0,
            cooccur: Vec::new(),
            agency_bias: AgencyFrame::None,
        }
    }

    pub fn trustworthiness(&self) -> f64 {
        (1.0 - self.bs) * 0.7 + (self.winding as f64 / 6.0) * 0.3
    }

    pub fn id_hex(&self) -> String {
        self.id.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn promote(&mut self) {
        self.count += 1;
        if self.winding < 6 {
            self.winding += 1;
            self.bs = (self.bs * 0.9).max(0.0);
        }
    }

    /// CCP: weight = count × recency_factor × cluster_density
    /// High weight = stable slot → compress hard.
    /// Low weight / fast-changing = volatile → don't compress (carries actual info).
    pub fn weight(&self, current_turn: u32) -> f64 {
        let recency = if current_turn == 0 { 1.0 } else {
            1.0 / (1.0 + (current_turn - self.last_seen_turn) as f64 * 0.1)
        };
        let cluster_density = (self.cooccur.len() as f64 + 1.0).ln();
        self.count as f64 * recency * cluster_density
    }

    /// Record co-occurrence with another slot (by id).
    pub fn observe_cooccur(&mut self, other_id: [u8; 32]) {
        if !self.cooccur.contains(&other_id) {
            self.cooccur.push(other_id);
        }
    }

    /// Update agency bias — dominant frame wins.
    pub fn observe_frame(&mut self, frame: AgencyFrame) {
        if frame != AgencyFrame::None {
            self.agency_bias = frame;
        }
    }
}

pub fn tokens_to_radius(tokens: usize) -> f64 {
    (tokens as f64) / TAU
}
