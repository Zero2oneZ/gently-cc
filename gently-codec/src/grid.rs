use crate::pin::Pin;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeKind {
    Directed,  // [*A → *B]
    Merge,     // [*A ≡ *B] — same referent
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,  // pin name
    pub to: String,    // pin name
    pub kind: EdgeKind,
}

/// Session-scoped directed graph of named concept pins.
/// Coordinates emerge from topology — no x/y needed up front.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinGrid {
    pub pins: HashMap<String, Pin>,  // name → Pin
    pub edges: Vec<Edge>,
    pub turn: u32,
}

impl PinGrid {
    pub fn new() -> Self {
        PinGrid { pins: HashMap::new(), edges: Vec::new(), turn: 0 }
    }

    /// Define a pin. Returns true if new, false if already existed.
    pub fn define(&mut self, name: &str, label: &str) -> bool {
        if self.pins.contains_key(name) {
            return false;
        }
        self.pins.insert(name.to_string(), Pin::new(name, label, self.turn));
        true
    }

    /// Reference a pin — increments ref_count, returns Some(label) or None.
    pub fn reference(&mut self, name: &str) -> Option<String> {
        if let Some(pin) = self.pins.get_mut(name) {
            if pin.tombstone { return None; }
            pin.ref_count += 1;
            Some(pin.label.clone())
        } else {
            None
        }
    }

    /// Add a directed edge.
    pub fn add_edge(&mut self, from: &str, to: &str, kind: EdgeKind) {
        // Only add if both pins exist
        if self.pins.contains_key(from) && self.pins.contains_key(to) {
            if !self.edges.iter().any(|e| e.from == from && e.to == to) {
                self.edges.push(Edge { from: from.to_string(), to: to.to_string(), kind });
            }
        }
    }

    /// Tombstone a pin — deprecated, address stays valid, refs return None.
    pub fn deprecate(&mut self, name: &str) -> bool {
        if let Some(pin) = self.pins.get_mut(name) {
            pin.tombstone = true;
            true
        } else {
            false
        }
    }

    /// Promote turn counter (call once per user message).
    pub fn advance_turn(&mut self) {
        self.turn += 1;
    }

    /// Pins that have crossed the break-even threshold (ref_count >= 5).
    pub fn mature_pins(&self) -> Vec<&Pin> {
        let mut v: Vec<&Pin> = self.pins.values()
            .filter(|p| !p.tombstone && p.ref_count >= 5)
            .collect();
        v.sort_by(|a, b| b.ref_count.cmp(&a.ref_count));
        v
    }

    /// Total tokens saved across all pins this session.
    pub fn total_saved(&self) -> usize {
        self.pins.values().map(|p| p.total_saved()).sum()
    }

    /// All pins sorted by ref_count descending, tombstones last.
    pub fn sorted_pins(&self) -> Vec<&Pin> {
        let mut v: Vec<&Pin> = self.pins.values().collect();
        v.sort_by(|a, b| {
            a.tombstone.cmp(&b.tombstone)
                .then(b.ref_count.cmp(&a.ref_count))
        });
        v
    }

    /// Edges from a given pin name.
    pub fn edges_from(&self, name: &str) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from == name).collect()
    }
}
