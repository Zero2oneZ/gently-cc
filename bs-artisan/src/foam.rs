use crate::barf::{barf, BarfQuery, BarfResult};
use crate::torus::Torus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorusBlend {
    pub a: [u8; 32],
    pub b: [u8; 32],
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foam {
    pub tori: HashMap<String, Torus>,
    pub blends: Vec<TorusBlend>,
    pub genesis: [u8; 32],
    #[serde(default)]
    pub turn: u32,
}

impl Foam {
    pub fn new() -> Self {
        let genesis = blake3::hash(b"gently-foam-genesis").into();
        Foam { tori: HashMap::new(), blends: Vec::new(), genesis, turn: 0 }
    }

    pub fn advance_turn(&mut self) {
        self.turn += 1;
    }

    /// Wire 2: record co-occurrence between two tori by label.
    /// Both tori must already exist. Bidirectional.
    pub fn observe_cooccur(&mut self, label_a: &str, label_b: &str) {
        let id_a: [u8; 32] = blake3::hash(label_a.as_bytes()).into();
        let id_b: [u8; 32] = blake3::hash(label_b.as_bytes()).into();
        let hex_a: String = id_a.iter().map(|b| format!("{:02x}", b)).collect();
        let hex_b: String = id_b.iter().map(|b| format!("{:02x}", b)).collect();

        if let Some(ta) = self.tori.get_mut(&hex_a) {
            ta.observe_cooccur(id_b);
            ta.last_seen_turn = self.turn;
        }
        if let Some(tb) = self.tori.get_mut(&hex_b) {
            tb.observe_cooccur(id_a);
            tb.last_seen_turn = self.turn;
        }
    }

    /// Wire 3: record agency frame for a torus by label.
    pub fn observe_frame(&mut self, label: &str, frame: crate::torus::AgencyFrame) {
        let id: [u8; 32] = blake3::hash(label.as_bytes()).into();
        let hex: String = id.iter().map(|b| format!("{:02x}", b)).collect();
        if let Some(t) = self.tori.get_mut(&hex) {
            t.observe_frame(frame);
            t.last_seen_turn = self.turn;
        }
    }

    pub fn insert(&mut self, label: &str, tokens: usize) -> [u8; 32] {
        let t = Torus::new(label, tokens);
        let id = t.id;
        let key = t.id_hex();
        if let Some(existing) = self.tori.get_mut(&key) {
            existing.promote();
        } else {
            self.tori.insert(key, t);
        }
        id
    }

    pub fn get_by_hex(&self, hex: &str) -> Option<&Torus> {
        self.tori.get(hex)
    }

    pub fn blend(&mut self, a_hex: &str, b_hex: &str, strength: f64) {
        let a = match self.tori.get(a_hex) { Some(t) => t.id, None => return };
        let b = match self.tori.get(b_hex) { Some(t) => t.id, None => return };

        if let Some(existing) = self.blends.iter_mut().find(|bl| bl.a == a && bl.b == b) {
            existing.strength = (existing.strength + strength).min(1.0);
            return;
        }
        self.blends.push(TorusBlend { a, b, strength });

        // wire bidirectional connections
        let a_hex = a_hex.to_string();
        let b_hex = b_hex.to_string();
        if let Some(ta) = self.tori.get_mut(&a_hex) {
            if !ta.connections.contains(&b) { ta.connections.push(b); }
        }
        if let Some(tb) = self.tori.get_mut(&b_hex) {
            if !tb.connections.contains(&a) { tb.connections.push(a); }
        }
    }

    pub fn query(&self, text: &str, max: usize) -> Vec<BarfResult> {
        let by_id: HashMap<[u8; 32], Torus> = self
            .tori.values()
            .map(|t| (t.id, t.clone()))
            .collect();
        let q = BarfQuery::new(text).with_turn(self.turn);
        let mut q = q;
        q.max_results = max;
        barf(&q, &by_id)
    }

    pub fn stats(&self) -> FoamStats {
        let total_trust: f64 = self.tori.values().map(|t| t.trustworthiness()).sum();
        let avg_trust = if self.tori.is_empty() { 0.0 } else { total_trust / self.tori.len() as f64 };
        FoamStats {
            tori_count: self.tori.len(),
            blend_count: self.blends.len(),
            avg_trust,
        }
    }
}

pub struct FoamStats {
    pub tori_count: usize,
    pub blend_count: usize,
    pub avg_trust: f64,
}

pub fn foam_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".gently").join("foam.json")
}

pub fn load_foam() -> Foam {
    let path = foam_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_else(|_| Foam::new())
    } else {
        Foam::new()
    }
}

pub fn save_foam(foam: &Foam) {
    let path = foam_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(foam) {
        let _ = std::fs::write(path, data);
    }
}
