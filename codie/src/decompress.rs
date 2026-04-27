//! CODIE decompression: glyph form → readable keyword form.
//! Best-effort: reverses glyph substitution. Squeezed identifiers stay squeezed.

use crate::glyph::from_glyph;
use crate::operator::expand_operators;

pub fn decompress(text: &str) -> String {
    // First expand any CCP operator glyphs back to prefix form
    let op_expanded = expand_operators(text);

    let mut out = String::with_capacity(op_expanded.len() * 2);
    let mut chars = op_expanded.chars().peekable();
    let mut need_space = false;

    while let Some(c) = chars.next() {
        let s: String = std::iter::once(c).collect();

        if let Some(kw) = from_glyph(&s) {
            if need_space { out.push(' '); }
            out.push_str(kw);
            need_space = true;
            continue;
        }

        match c {
            '⟨' => {
                if need_space { out.push(' '); }
                out.push_str("{\n  ");
                need_space = false;
            }
            '⟩' => {
                out.push_str("\n}");
                need_space = false;
            }
            ' ' => {
                out.push(' ');
                need_space = false;
            }
            _ => {
                if need_space && !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push(c);
                need_space = false;
            }
        }
    }

    out
}
