use crate::torus::Torus;
use std::collections::HashMap;

pub struct BarfQuery {
    pub hash: [u8; 32],
    pub max_results: usize,
    pub min_trust: f64,
    pub current_turn: u32,
}

#[derive(Debug)]
pub struct BarfResult {
    pub torus: Torus,
    pub distance: f64,
    pub score: f64,
}

impl BarfQuery {
    pub fn new(text: &str) -> Self {
        BarfQuery {
            hash: blake3::hash(text.as_bytes()).into(),
            max_results: 5,
            min_trust: 0.0,
            current_turn: 0,
        }
    }

    pub fn with_turn(mut self, turn: u32) -> Self {
        self.current_turn = turn;
        self
    }
}

pub fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> f64 {
    let bits: u32 = a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum();
    bits as f64 / 256.0
}

/// Wire 4: weight-driven scoring replaces trustworthiness().
/// weight(turn) = count × recency × cluster_density
/// Stable, frequently co-occurring tori score higher. Volatile slots stay low.
pub fn barf(query: &BarfQuery, tori: &HashMap<[u8; 32], Torus>) -> Vec<BarfResult> {
    let mut results: Vec<BarfResult> = tori
        .values()
        .map(|t| {
            let mut dist = xor_distance(&query.hash, &t.id);
            // Connected tori (via blends/co-occurrence) are pulled closer
            let conn_boost = t.connections.len().min(3) as u32;
            dist *= 0.5f64.powi(conn_boost as i32);
            // Wire 4: use weight() not trustworthiness()
            let score = (1.0 - dist) * t.weight(query.current_turn);
            BarfResult { torus: t.clone(), distance: dist, score }
        })
        .filter(|r| r.score > 0.0)
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap()); // sort by score desc
    results.truncate(query.max_results);
    results
}
