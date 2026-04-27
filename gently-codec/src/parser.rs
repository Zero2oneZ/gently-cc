//! Inline CCP pin syntax parser.
//!
//! Scans text for pin declarations and references, applies them to a PinGrid,
//! and emits compressed output. This is layer 3 (semantic) of the codec stack.
//!
//! Syntax:
//!   [*name: label text]    — declare pin "name" with label "label text"
//!   *name                  — reference pin (replaced by label on expand, *name on compress)
//!   [*a → *b]              — directed edge between pins a and b
//!   [*a ≡ *b]              — merge: a and b are the same referent
//!   [*a: ~]                — tombstone pin a

use crate::grid::{EdgeKind, PinGrid};
use crate::scope::{ScopeLevel, ScopedGrid};

pub struct ParseResult {
    pub output: String,
    pub pins_defined: Vec<(String, String)>,   // (name, label)
    pub pins_referenced: Vec<String>,
    pub edges_added: usize,
    /// Wire 3: (pin_name, operator_char) for each reference that fired under a CCP frame
    pub agency_observations: Vec<(String, char)>,
    /// Wire 2: pairs of pin names that co-fired in this prompt
    pub cooccur_pairs: Vec<(String, String)>,
}

// We use a simple hand-rolled scanner to avoid regex dependency on hot path.
// Regex used only in tests / verification.

pub fn compress_pins(text: &str, grid: &mut PinGrid) -> ParseResult {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let mut pins_defined: Vec<(String, String)> = Vec::new();
    let mut pins_referenced = Vec::new();
    let mut edges_added = 0;

    while i < chars.len() {
        // Check for [ — might be a declaration/edge
        if chars[i] == '[' && i + 1 < chars.len() && chars[i+1] == '*' {
            if let Some((consumed, directive)) = parse_bracket_directive(&chars[i..]) {
                match directive {
                    Directive::Define(name, label) => {
                        let is_new = grid.define(&name, &label);
                        if is_new {
                            pins_defined.push((name.clone(), label.clone()));
                        }
                        // Emit just *name — the label is now in the grid
                        out.push('*');
                        out.push_str(&name);
                    }
                    Directive::Edge(a, b) => {
                        grid.add_edge(&a, &b, EdgeKind::Directed);
                        edges_added += 1;
                        // Emit compact form
                        out.push('*');
                        out.push_str(&a);
                        out.push_str("→*");
                        out.push_str(&b);
                    }
                    Directive::Merge(a, b) => {
                        grid.add_edge(&a, &b, EdgeKind::Merge);
                        edges_added += 1;
                        out.push('*');
                        out.push_str(&a);
                        out.push_str("≡*");
                        out.push_str(&b);
                    }
                    Directive::Tombstone(name) => {
                        grid.deprecate(&name);
                        out.push_str("†*");
                        out.push_str(&name);
                    }
                }
                i += consumed;
                continue;
            }
        }

        // Check for standalone *name reference
        if chars[i] == '*' && i + 1 < chars.len() && chars[i+1].is_alphanumeric() {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[start..j].iter().collect();
            if grid.pins.contains_key(&name) {
                grid.reference(&name);
                pins_referenced.push(name.clone());
                // Stays as *name on wire — receiver expands
                out.push('*');
                out.push_str(&name);
                i = j;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    let mut cooccur_pairs = Vec::new();
    for (idx, a) in pins_referenced.iter().enumerate() {
        for b in pins_referenced.iter().skip(idx + 1) {
            cooccur_pairs.push((a.clone(), b.clone()));
        }
    }
    ParseResult { output: out, pins_defined, pins_referenced, edges_added,
                  agency_observations: vec![], cooccur_pairs }
}

/// CCP operator chars — used for agency frame detection.
const OP_CHARS: &[char] = &['?', '>', '<', '!', '=', '~', ':',
    '⟐', '⟹', '⟸', '⟑', '≡', '≈', '∷'];  // compressed forms too

fn char_to_frame(c: char) -> Option<char> {
    match c {
        '?' | '⟐' => Some('?'),
        '>' | '⟹' => Some('>'),
        '<' | '⟸' => Some('<'),
        '!' | '⟑' => Some('!'),
        '=' | '≡' => Some('='),
        '~' | '≈' => Some('~'),
        ':' | '∷' => Some(':'),
        _ => None,
    }
}

/// Scoped compress: declarations go to chat level, references walk the full tree.
/// Wire 2: tracks co-occurrence pairs.
/// Wire 3: tracks which operator frame each pin fires under.
pub fn compress_scoped(text: &str, sg: &mut ScopedGrid) -> ParseResult {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let mut pins_defined: Vec<(String, String)> = Vec::new();
    let mut pins_referenced = Vec::new();
    let mut edges_added = 0;
    let mut agency_observations: Vec<(String, char)> = Vec::new();
    let mut active_frame: Option<char> = None;

    while i < chars.len() {
        // Track active CCP operator frame — wire 3
        // A frame is active from an operator char until the next whitespace boundary
        if let Some(frame) = char_to_frame(chars[i]) {
            active_frame = Some(frame);
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // Reset frame on newline
        if chars[i] == '\n' {
            active_frame = None;
        }

        if chars[i] == '[' && i + 1 < chars.len() && chars[i+1] == '*' {
            if let Some((consumed, directive)) = parse_bracket_directive(&chars[i..]) {
                match directive {
                    Directive::Define(name, label) => {
                        let is_new = sg.define_at(ScopeLevel::Chat, &name, &label);
                        if is_new { pins_defined.push((name.clone(), label.clone())); }
                        out.push('*');
                        out.push_str(&name);
                    }
                    Directive::Edge(a, b) => {
                        sg.add_edge(&a, &b, EdgeKind::Directed);
                        edges_added += 1;
                        out.push('*'); out.push_str(&a);
                        out.push_str("→*"); out.push_str(&b);
                    }
                    Directive::Merge(a, b) => {
                        sg.add_edge(&a, &b, EdgeKind::Merge);
                        edges_added += 1;
                        out.push('*'); out.push_str(&a);
                        out.push_str("≡*"); out.push_str(&b);
                    }
                    Directive::Tombstone(name) => {
                        sg.chat.deprecate(&name);
                        out.push_str("†*"); out.push_str(&name);
                    }
                }
                i += consumed;
                continue;
            }
        }

        if chars[i] == '*' && i + 1 < chars.len() && chars[i+1].is_alphanumeric() {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') { j += 1; }
            let name: String = chars[start..j].iter().collect();
            if sg.resolve(&name).is_some() {
                sg.reference(&name);
                pins_referenced.push(name.clone());
                // Wire 3: record agency frame for this reference
                if let Some(frame) = active_frame {
                    agency_observations.push((name.clone(), frame));
                }
                out.push('*'); out.push_str(&name);
                i = j;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    // Wire 2: build co-occurrence pairs from all pins that fired in this prompt
    let mut cooccur_pairs = Vec::new();
    for (idx, a) in pins_referenced.iter().enumerate() {
        for b in pins_referenced.iter().skip(idx + 1) {
            cooccur_pairs.push((a.clone(), b.clone()));
        }
    }

    ParseResult { output: out, pins_defined, pins_referenced, edges_added, agency_observations, cooccur_pairs }
}

/// Scoped expand: resolves *name through full tree.
pub fn expand_scoped(text: &str, sg: &ScopedGrid) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '*' && i + 1 < chars.len() && chars[i+1].is_alphanumeric() {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') { j += 1; }
            let name: String = chars[start..j].iter().collect();
            if let Some(r) = sg.resolve(&name) {
                out.push('[');
                out.push_str(&r.pin.label);
                out.push(']');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Expand *name references back to full labels (for human-readable output).
pub fn expand_pins(text: &str, grid: &PinGrid) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '*' && i + 1 < chars.len() && chars[i+1].is_alphanumeric() {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[start..j].iter().collect();
            if let Some(pin) = grid.pins.get(&name) {
                out.push('[');
                out.push_str(&pin.label);
                out.push(']');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

enum Directive {
    Define(String, String),   // [*name: label]
    Edge(String, String),     // [*a → *b]
    Merge(String, String),    // [*a ≡ *b]
    Tombstone(String),        // [*a: ~]
}

/// Parse a bracket directive starting at chars[0] == '['.
/// Returns (chars_consumed, Directive) or None if not a valid directive.
fn parse_bracket_directive(chars: &[char]) -> Option<(usize, Directive)> {
    // Must start with [*
    if chars.len() < 4 || chars[0] != '[' || chars[1] != '*' {
        return None;
    }

    // Read first pin name
    let mut i = 2;
    let name_start = i;
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    if i == name_start { return None; }
    let name_a: String = chars[name_start..i].iter().collect();

    // Skip whitespace
    while i < chars.len() && chars[i] == ' ' { i += 1; }

    if i >= chars.len() { return None; }

    match chars[i] {
        // [*name: label] or [*name: ~]
        ':' => {
            i += 1;
            while i < chars.len() && chars[i] == ' ' { i += 1; }

            // Find closing ]
            let content_start = i;
            while i < chars.len() && chars[i] != ']' { i += 1; }
            if i >= chars.len() { return None; }

            let content: String = chars[content_start..i].iter().collect();
            let consumed = i + 1; // include ]

            if content.trim() == "~" {
                Some((consumed, Directive::Tombstone(name_a)))
            } else {
                Some((consumed, Directive::Define(name_a, content.trim().to_string())))
            }
        }

        // [*a → *b]
        '→' => {
            i += 1;
            while i < chars.len() && chars[i] == ' ' { i += 1; }
            if i >= chars.len() || chars[i] != '*' { return None; }
            i += 1;
            let b_start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
            let name_b: String = chars[b_start..i].iter().collect();
            while i < chars.len() && chars[i] == ' ' { i += 1; }
            if i >= chars.len() || chars[i] != ']' { return None; }
            Some((i + 1, Directive::Edge(name_a, name_b)))
        }

        // [*a ≡ *b]
        '≡' => {
            i += 1;
            while i < chars.len() && chars[i] == ' ' { i += 1; }
            if i >= chars.len() || chars[i] != '*' { return None; }
            i += 1;
            let b_start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
            let name_b: String = chars[b_start..i].iter().collect();
            while i < chars.len() && chars[i] == ' ' { i += 1; }
            if i >= chars.len() || chars[i] != ']' { return None; }
            Some((i + 1, Directive::Merge(name_a, name_b)))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> PinGrid { PinGrid::new() }

    #[test]
    fn define_and_reference() {
        let mut g = fresh();
        let r = compress_pins("[*auth: authentication and session management]", &mut g);
        assert!(g.pins.contains_key("auth"), "pin not defined");
        assert!(r.output.contains("*auth"), "got: {}", r.output);
        assert!(!r.output.contains("authentication"), "label dropped from wire");
        assert!(r.pins_defined.iter().any(|(n,_)| n == "auth"), "should be in pins_defined");

        // reference it
        let r2 = compress_pins("the *auth system uses JWT", &mut g);
        assert!(r2.pins_referenced.contains(&"auth".to_string()));
        assert_eq!(g.pins["auth"].ref_count, 1);
    }

    #[test]
    fn edge_syntax() {
        let mut g = fresh();
        compress_pins("[*auth: auth flow]", &mut g);
        compress_pins("[*db: database layer]", &mut g);
        let r = compress_pins("[*auth → *db]", &mut g);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(r.edges_added, 1);
    }

    #[test]
    fn tombstone() {
        let mut g = fresh();
        compress_pins("[*old: old system]", &mut g);
        compress_pins("[*old: ~]", &mut g);
        assert!(g.pins["old"].tombstone);
        // reference returns none for tombstoned pins
        assert!(g.reference("old").is_none());
    }

    #[test]
    fn expand_roundtrip() {
        let mut g = fresh();
        compress_pins("[*auth: authentication and session management]", &mut g);
        let compressed = compress_pins("validate *auth token", &mut g);
        let expanded = expand_pins(&compressed.output, &g);
        assert!(expanded.contains("authentication and session management"), "got: {}", expanded);
    }

    #[test]
    fn tokens_saved() {
        let mut g = fresh();
        compress_pins("[*auth: the complete authentication and authorization subsystem]", &mut g);
        // 5 references crosses break-even
        for _ in 0..5 {
            compress_pins("check *auth before proceeding", &mut g);
        }
        let saved = g.pins["auth"].total_saved();
        assert!(saved > 0, "should save tokens: {}", saved);
    }
}
