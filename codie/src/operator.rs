//! CCP prefix operators — layer 2 structural compression.
//! Applied BEFORE the lexical glyph pass.
//!
//! First character = mode. Commits the parser before content arrives.
//! Higher compression ratio than lexical because grammar repeats more than nouns.

/// CCP operator prefix → compact form mapping.
/// The compact glyph is emitted to the wire; the receiver expands it.
pub const OPERATORS: &[(&str, &str, &str)] = &[
    //  prefix  glyph   meaning
    ("?",  "⟐",  "query/opinion — sender in motion"),
    ("!",  "⟑",  "assertion — sender planted"),
    (">",  "⟹",  "command — caller frame, induces motion"),
    ("<",  "⟸",  "return — callee, returning agency"),
    ("=",  "≡",  "settled truth — both committed"),
    ("~",  "≈",  "fuzzy/exploratory — low commitment"),
    (":",  "∷",  "definition — establishes referent, binds slot"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedOp {
    pub op: &'static str,
    pub glyph: &'static str,
    pub payload: String,
}

/// Parse a line/token starting with a CCP operator.
/// Returns Some(ParsedOp) if first char matches, None otherwise.
pub fn parse_operator(text: &str) -> Option<ParsedOp> {
    let first = text.chars().next()?;
    let first_str = &text[..first.len_utf8()];
    for &(op, glyph, _) in OPERATORS {
        if first_str == op {
            let payload = text[first.len_utf8()..].trim().to_string();
            return Some(ParsedOp { op, glyph, payload });
        }
    }
    None
}

/// Apply operator compression to a full text block.
/// Lines starting with a CCP operator get the operator replaced by its glyph.
/// Non-operator lines pass through unchanged (lexical pass handles them).
pub fn compress_operators(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(parsed) = parse_operator(trimmed) {
            // Preserve leading whitespace
            let indent_len = line.len() - trimmed.len();
            out.push_str(&line[..indent_len]);
            out.push_str(parsed.glyph);
            out.push_str(&parsed.payload);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // trim trailing newline if original didn't have one
    if !text.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Expand operator glyphs back to their prefix form (for decompress path).
pub fn expand_operators(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        let mut matched = false;
        for &(op, glyph, _) in OPERATORS {
            if trimmed.starts_with(glyph) {
                let indent_len = line.len() - trimmed.len();
                out.push_str(&line[..indent_len]);
                out.push_str(op);
                out.push_str(&trimmed[glyph.len()..]);
                matched = true;
                break;
            }
        }
        if !matched {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !text.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query() {
        let p = parse_operator("?what is the user count").unwrap();
        assert_eq!(p.op, "?");
        assert_eq!(p.glyph, "⟐");
        assert_eq!(p.payload, "what is the user count");
    }

    #[test]
    fn parses_command() {
        let p = parse_operator(">fetch all users from database").unwrap();
        assert_eq!(p.op, ">");
        assert_eq!(p.glyph, "⟹");
    }

    #[test]
    fn compress_expand_roundtrip() {
        let src = "?what is the count\n!the count is 42\n>run the migration";
        let compressed = compress_operators(src);
        let expanded = expand_operators(&compressed);
        assert_eq!(expanded, src);
    }

    #[test]
    fn non_operator_passthrough() {
        let src = "just a normal line";
        assert_eq!(compress_operators(src).trim(), src);
    }
}
