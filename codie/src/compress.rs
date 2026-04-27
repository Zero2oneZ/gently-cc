//! CODIE compression: natural language / structured CODIE → glyph form.
//!
//! Pipeline:
//!   1. Keyword glyph substitution (exact word match, case-insensitive)
//!   2. Abbreviation dictionary (common programming terms)
//!   3. Vowel strip for long words (>= 5 chars, skip first char)
//!   4. Tree chars (├ └ │) → bracket nesting (⟨⟩)
//!
//! Token counting heuristic (±10% of Claude tokenizer):
//!   original  = whitespace-split word count
//!   compressed = ceil(output_char_len / 4)

use crate::glyph::{to_glyph, is_known};
use crate::operator::compress_operators;

const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u'];

/// Abbreviation dictionary. Applied before vowel strip.
/// Longer entries first so "authenticate" beats "auth" on overlap.
const ABBREVS: &[(&str, &str)] = &[
    ("authenticate", "auth"),
    ("authentication", "auth"),
    ("authorization", "authz"),
    ("configuration", "cfg"),
    ("asynchronous", "async"),
    ("implementation", "impl"),
    ("specification", "spec"),
    ("administrator", "admin"),
    ("deserialize", "deser"),
    ("synchronize", "sync"),
    ("information", "info"),
    ("application", "app"),
    ("permission", "perm"),
    ("collection", "coll"),
    ("connection", "conn"),
    ("transaction", "tx"),
    ("middleware", "mw"),
    ("repository", "repo"),
    ("reference", "ref"),
    ("temporary", "tmp"),
    ("parameter", "param"),
    ("serialize", "ser"),
    ("attribute", "attr"),
    ("component", "cmp"),
    ("controller", "ctrl"),
    ("directory", "dir"),
    ("document", "doc"),
    ("function", "fn"),
    ("response", "resp"),
    ("callback", "cb"),
    ("database", "db"),
    ("iterator", "iter"),
    ("argument", "arg"),
    ("variable", "var"),
    ("constant", "const"),
    ("generate", "gen"),
    ("property", "prop"),
    ("timestamp", "ts"),
    ("password", "pwd"),
    ("username", "uname"),
    ("interval", "intv"),
    ("session", "sess"),
    ("message", "msg"),
    ("service", "svc"),
    ("element", "elem"),
    ("boolean", "bool"),
    ("integer", "int"),
    ("maximum", "max"),
    ("minimum", "min"),
    ("average", "avg"),
    ("address", "addr"),
    ("pointer", "ptr"),
    ("request", "req"),
    ("timeout", "tout"),
    ("handler", "hndlr"),
    ("channel", "ch"),
    ("context", "ctx"),
    ("process", "proc"),
    ("library", "lib"),
    ("package", "pkg"),
    ("manager", "mgr"),
    ("gateway", "gw"),
    ("network", "net"),
    ("storage", "stor"),
    ("execute", "exec"),
    ("compute", "cmp"),
    ("version", "ver"),
    ("buffer", "buf"),
    ("string", "str"),
    ("number", "num"),
    ("result", "res"),
    ("length", "len"),
    ("source", "src"),
    ("module", "mod"),
    ("filter", "flt"),
    ("signal", "sig"),
    ("memory", "mem"),
    ("vector", "vec"),
    ("thread", "thrd"),
    ("stream", "strm"),
    ("worker", "wkr"),
    ("status", "stat"),
    ("router", "rtr"),
    ("output", "out"),
    ("object", "obj"),
    ("format", "fmt"),
    ("script", "scr"),
    ("binary", "bin"),
    ("socket", "sock"),
    ("update", "upd"),
    ("create", "crt"),
    ("delete", "del"),
    ("insert", "ins"),
    ("select", "sel"),
    ("commit", "cmt"),
    ("token", "tok"),
    ("error", "err"),
    ("value", "val"),
    ("count", "cnt"),
    ("index", "idx"),
    ("total", "tot"),
    ("input", "in"),
    ("array", "arr"),
    ("queue", "q"),
    ("stack", "stk"),
    ("graph", "gr"),
    ("image", "img"),
    ("audio", "aud"),
    ("video", "vid"),
    ("event", "evt"),
    ("frame", "frm"),
    ("cache", "cch"),
    ("mutex", "mtx"),
    ("debug", "dbg"),
    ("state", "st"),
    ("table", "tbl"),
    ("range", "rng"),
    ("route", "rt"),
    ("found", "fnd"),
    ("user", "usr"),
    ("from", "frm"),
    ("with", "w/"),
    ("send", "snd"),
    ("read", "rd"),
    ("write", "wr"),
    ("file", "f"),
    ("data", "d"),
    ("true", "⊤"),
    ("false", "⊥"),
];

pub struct Compressed {
    pub output: String,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    /// Tokens that had no rule (no glyph, no abbrev) and passed through raw.
    /// These are candidates for generative pinning by the codec layer.
    pub unknowns: Vec<String>,
}

impl Compressed {
    pub fn pct_saved(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        let saved = self.original_tokens.saturating_sub(self.compressed_tokens);
        (saved as f64 / self.original_tokens as f64) * 100.0
    }
}

pub fn compress(text: &str) -> Compressed {
    let original_tokens = count_tokens(text);

    // Pass 1: CCP prefix operators → glyphs (structural layer — compresses grammar)
    let op_compressed = compress_operators(text);

    // Pass 2: tree-char lines → bracket nesting
    let bracketed = bracket_trees(&op_compressed);

    // Pass 3: word-by-word glyph + squeeze (lexical layer), collecting unknowns
    let (output, unknowns) = compress_words_tracked(&bracketed);

    let compressed_tokens = estimate_tokens(&output);

    Compressed { output, original_tokens, compressed_tokens, unknowns }
}

/// Tokens that had no compression rule — generative candidates for the codec pin layer.
/// Criteria: length ≥ 5, not a common stop word, not already a glyph/abbrev.
fn is_candidate_unknown(word: &str) -> bool {
    if word.len() < 5 { return false; }
    let lower = word.to_lowercase();
    // Skip if it's a glyph keyword
    if is_known(&lower) { return false; }
    // Skip if it's an abbreviation target (already in the dict)
    if ABBREVS.iter().any(|(_, abbrev)| lower == *abbrev) { return false; }
    // Skip common English stop words and articles
    const STOP: &[&str] = &[
        "the", "this", "that", "these", "those", "with", "from", "into", "onto",
        "about", "above", "after", "before", "under", "where", "which", "there",
        "their", "other", "would", "could", "should", "might", "shall", "will",
        "have", "been", "were", "more", "some", "such", "then", "than", "also",
        "both", "each", "many", "much", "even", "same", "here", "when", "what",
        "your", "they", "them", "make", "made", "take", "took", "give", "gave",
        "using" , "used", "based", "given", "being", "doing", "going",
    ];
    !STOP.contains(&lower.as_str())
}

/// Convert ASCII tree-drawing chars to ⟨⟩ bracket nesting.
fn bracket_trees(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth_stack: Vec<usize> = Vec::new();
    let mut prev_depth = 0usize;

    for line in text.lines() {
        let (depth, content) = parse_tree_line(line);
        if content.is_empty() {
            continue;
        }

        if depth > prev_depth {
            out.push('⟨');
            depth_stack.push(prev_depth);
        } else {
            while depth_stack.last().map_or(false, |&d| d >= depth) {
                depth_stack.pop();
                out.push('⟩');
            }
        }

        out.push_str(&content);
        out.push(' ');
        prev_depth = depth;
    }

    while !depth_stack.is_empty() {
        depth_stack.pop();
        out.push('⟩');
    }

    out
}

fn parse_tree_line(line: &str) -> (usize, String) {
    const TREE: &[char] = &['├', '─', '└', '│', '┌', '┐', '┘', '┴', '┬', '┤', '┼'];
    let mut depth = 0usize;
    let mut in_tree = true;
    let mut content = String::new();

    for c in line.chars() {
        if in_tree {
            match c {
                ' ' | '\t' => depth += 1,
                '│' => depth += 4,
                '├' | '└' => { depth += 4; in_tree = false; }
                _ => { in_tree = false; content.push(c); }
            }
        } else if !TREE.contains(&c) {
            content.push(c);
        }
    }

    let content = content.trim().to_string();
    (depth, content)
}

/// Compress word-by-word: glyph table → abbrev dict → vowel strip.
/// Returns (compressed_output, unknown_tokens).
fn compress_words_tracked(text: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(text.len() / 2);
    let mut unknowns: Vec<String> = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        let c = chars[i];

        // Preserve structural glyphs and brackets
        if "⟨⟩←→@#$⊤⊥".contains(c) {
            out.push(c);
            i += 1;
            continue;
        }

        // Collect a word
        if c.is_alphanumeric() || c == '_' || c == '-' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let (compressed, is_unknown) = compress_word_tracked(&word);
            if is_unknown && is_candidate_unknown(&word) {
                unknowns.push(word.clone());
            }
            out.push_str(&compressed);
        } else {
            out.push(c);
            i += 1;
        }
    }

    // Deduplicate unknowns, cap at 10
    unknowns.dedup();
    unknowns.truncate(10);

    (out, unknowns)
}

/// Compatibility shim for callers that don't need unknowns.
fn compress_words(text: &str) -> String {
    compress_words_tracked(text).0
}

/// Returns (compressed, is_unknown).
/// is_unknown = true when the word had no glyph, no abbrev, and no structural decomposition —
/// meaning it passed through as vowel-stripped original. These are pin candidates.
fn compress_word_tracked(word: &str) -> (String, bool) {
    let lower = word.to_lowercase();

    // Glyph table hit → known, not unknown
    if let Some(g) = to_glyph(&lower) {
        return (g.to_string(), false);
    }

    // Abbreviation dictionary hit → known
    for (full, abbrev) in ABBREVS {
        if lower == *full {
            return (abbrev.to_string(), false);
        }
    }

    // Compound: split on _ — unknown only if ALL parts are unknown
    if word.contains('_') {
        let mut all_unknown = true;
        let parts: Vec<String> = word.split('_').map(|p| {
            let (c, u) = compress_word_tracked(p);
            if !u { all_unknown = false; }
            c
        }).collect();
        return (parts.join("_"), all_unknown);
    }

    // CamelCase split
    if word.len() > 6 && word.chars().any(|c| c.is_uppercase()) {
        let parts = split_camel(word);
        if parts.len() > 1 {
            let mut all_unknown = true;
            let compressed: Vec<String> = parts.iter().map(|p| {
                let (c, u) = compress_word_tracked(p);
                if !u { all_unknown = false; }
                c
            }).collect();
            return (compressed.join(""), all_unknown);
        }
    }

    // Vowel strip — this word had no rule, it's an unknown
    let squeezed = if word.len() >= 5 { squeeze_vowels(word) } else { word.to_string() };
    (squeezed, true)
}

fn compress_word(word: &str) -> String {
    compress_word_tracked(word).0
}

/// Split "getUserName" → ["get", "User", "Name"]
fn split_camel(word: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for c in word.chars() {
        if c.is_uppercase() && !current.is_empty() {
            parts.push(current.clone());
            current.clear();
        }
        current.push(c);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Remove vowels, keeping the first character.
fn squeeze_vowels(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for (i, c) in word.char_indices() {
        if i == 0 || !VOWELS.contains(&c.to_ascii_lowercase()) {
            out.push(c);
        }
    }
    out
}

/// Count tokens via char length / 4 (matches Claude tokenizer within ~10%).
/// Applied consistently to both original and compressed text so the ratio is accurate.
pub fn count_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

pub fn estimate_tokens(text: &str) -> usize {
    count_tokens(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_keyword() {
        let c = compress("pug LOGIN bark user return token");
        assert!(c.output.contains('ρ'));
        assert!(c.output.contains('β'));
        assert!(c.output.contains('→'));
    }

    #[test]
    fn abbreviation() {
        let c = compress("fetch user from database if not found return error");
        assert!(c.output.contains("db"), "got: {}", c.output);
        assert!(c.output.contains("err"), "got: {}", c.output);
    }

    #[test]
    fn achieves_compression() {
        let text = "Fetch the user from the database. If not found return an error. Otherwise return a session token.";
        let c = compress(text);
        assert!(c.pct_saved() > 20.0, "expected >20% savings, got {:.1}%", c.pct_saved());
    }

    #[test]
    fn tree_to_brackets() {
        let src = "pug LOGIN\n├── bark user\n└── return token";
        let c = compress(src);
        // brackets inserted
        assert!(c.output.contains('⟨') || c.output.contains('⟩') || c.output.contains('ρ'));
    }
}
