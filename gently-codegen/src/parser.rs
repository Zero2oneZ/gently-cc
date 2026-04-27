// CODIE symbol string → AST Node
//
// Input format (two modes):
//   1. Symbol shorthand:  "κ[auth] ε[uid:UserId] β[db/users←uid] ⁇¬found→⊥ →token"
//   2. JSON DAG:          {"op":"Define","name":"auth",...}
//
// The parser is intentionally simple — it reads left to right, builds a Pipe
// of the top-level operations, and recurses into brackets for sub-expressions.

use crate::ast::{DefKind, Node, Param};

pub fn parse(input: &str) -> Node {
    let input = input.trim();

    // JSON DAG passthrough
    if input.starts_with('{') || input.starts_with('[') {
        if let Ok(n) = serde_json::from_str::<Node>(input) {
            return n;
        }
    }

    // CODIE symbol string — tokenise then build
    let tokens = tokenise(input);
    let mut pos = 0;
    let nodes = parse_sequence(&tokens, &mut pos);

    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        Node::Pipe { steps: nodes }
    }
}

// ── Tokeniser ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Glyph(char),       // κ β ε ⁇ → ∀ ⊥ ⊤ ∷ ⟹ ¬
    Bracket(String),   // contents of [...]
    Word(String),      // identifier, type name, path
    Colon,             // : (type separator in ε[name:Type])
    Arrow,             // ← (key separator in β[src←key])
    Newline,
}

fn tokenise(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars  = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            'κ'|'β'|'ε'|'⁇'|'∀'|'⊥'|'⊤'|'∷'|'¬' => tokens.push(Token::Glyph(c)),
            '→' => tokens.push(Token::Glyph('→')),
            '⟹' => tokens.push(Token::Glyph('⟹')),
            '[' => {
                let mut depth = 1;
                let mut inner = String::new();
                while let Some(c2) = chars.next() {
                    match c2 {
                        '[' => { depth += 1; inner.push(c2); }
                        ']' => { depth -= 1; if depth == 0 { break; } inner.push(c2); }
                        _   => inner.push(c2),
                    }
                }
                tokens.push(Token::Bracket(inner));
            }
            ':' => tokens.push(Token::Colon),
            '\n' => tokens.push(Token::Newline),
            ' '|'\t'|'\r' => {}
            _ => {
                let mut word = String::from(c);
                while let Some(&nc) = chars.peek() {
                    if nc.is_alphanumeric() || nc == '_' || nc == '/' || nc == '.' || nc == '<' || nc == '>' {
                        word.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Word(word));
            }
        }
    }
    tokens
}

// ── Recursive descent ────────────────────────────────────────

fn parse_sequence(tokens: &[Token], pos: &mut usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Newline => { *pos += 1; }
            _ => {
                if let Some(node) = parse_one(tokens, pos) {
                    nodes.push(node);
                } else {
                    break;
                }
            }
        }
    }
    nodes
}

fn parse_one(tokens: &[Token], pos: &mut usize) -> Option<Node> {
    if *pos >= tokens.len() { return None; }

    match &tokens[*pos].clone() {

        // κ[name] or κ[name:kind] — define
        Token::Glyph('κ') => {
            *pos += 1;
            let (name, kind) = bracket_name_kind(tokens, pos);
            let body = collect_body(tokens, pos);
            Some(Node::Define { name, kind, params: vec![], body })
        }

        // β[source] or β[source←key]
        Token::Glyph('β') => {
            *pos += 1;
            if let Some(Token::Bracket(inner)) = tokens.get(*pos) {
                let inner = inner.clone();
                *pos += 1;
                let (source, key) = if inner.contains('←') {
                    let mut parts = inner.splitn(2, '←');
                    let src = parts.next().unwrap_or("").trim().to_string();
                    let k   = parts.next().unwrap_or("").trim().to_string();
                    (src, Some(Box::new(Node::Atom { value: k })))
                } else {
                    (inner.trim().to_string(), None)
                };
                Some(Node::Fetch { source, key })
            } else {
                None
            }
        }

        // ε[name] or ε[name:Type]
        Token::Glyph('ε') => {
            *pos += 1;
            if let Some(Token::Bracket(inner)) = tokens.get(*pos) {
                let inner = inner.clone();
                *pos += 1;
                let (name, ty) = if inner.contains(':') {
                    let mut parts = inner.splitn(2, ':');
                    (parts.next().unwrap_or("").trim().to_string(),
                     Some(parts.next().unwrap_or("").trim().to_string()))
                } else {
                    (inner.trim().to_string(), None)
                };
                Some(Node::Bind { name, ty, value: None })
            } else {
                None
            }
        }

        // ⁇ — cond: next node is pred, then pipe-rest is then-branch
        Token::Glyph('⁇') => {
            *pos += 1;
            let pred = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "true".into() });
            let then = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "()".into() });
            Some(Node::Cond { pred: Box::new(pred), then: Box::new(then), else_: None })
        }

        // ¬  negate
        Token::Glyph('¬') => {
            *pos += 1;
            let inner = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "false".into() });
            Some(Node::Not { inner: Box::new(inner) })
        }

        // → return
        Token::Glyph('→') => {
            *pos += 1;
            let value = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "()".into() });
            Some(Node::Return { value: Box::new(value) })
        }

        // ⊥ fail
        Token::Glyph('⊥') => {
            *pos += 1;
            let reason = if let Some(Token::Bracket(r)) = tokens.get(*pos) {
                let r = r.clone(); *pos += 1; r
            } else {
                "Error".into()
            };
            Some(Node::Fail { reason })
        }

        // ⊤ ok/success
        Token::Glyph('⊤') => {
            *pos += 1;
            let value = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "()".into() });
            Some(Node::Ok { value: Box::new(value) })
        }

        // ∀[iter] — loop
        Token::Glyph('∀') => {
            *pos += 1;
            let iter = if let Some(Token::Bracket(inner)) = tokens.get(*pos) {
                let v = inner.clone(); *pos += 1;
                Node::Atom { value: v }
            } else {
                Node::Atom { value: "items".into() }
            };
            let body = collect_body(tokens, pos);
            Some(Node::Loop { iter: Box::new(iter), body })
        }

        // ∷  pipe (explicit)
        Token::Glyph('∷') => {
            *pos += 1;
            let steps = collect_body(tokens, pos);
            Some(Node::Pipe { steps })
        }

        // ⟹  flow with optional label
        Token::Glyph('⟹') => {
            *pos += 1;
            let label = if let Some(Token::Bracket(l)) = tokens.get(*pos) {
                let l = l.clone(); *pos += 1; Some(l)
            } else {
                None
            };
            let to = parse_one(tokens, pos).unwrap_or(Node::Atom { value: "next".into() });
            // from is whatever came before — caller wraps in Flow if needed
            Some(Node::Flow {
                from:  Box::new(Node::Atom { value: "_".into() }),
                to:    Box::new(to),
                label,
            })
        }

        // bare word — atom
        Token::Word(w) => {
            let w = w.clone();
            *pos += 1;
            Some(Node::Atom { value: w })
        }

        _ => { *pos += 1; None }
    }
}

fn bracket_name_kind(tokens: &[Token], pos: &mut usize) -> (String, DefKind) {
    if let Some(Token::Bracket(inner)) = tokens.get(*pos) {
        let inner = inner.clone();
        *pos += 1;
        if inner.contains(':') {
            let mut parts = inner.splitn(2, ':');
            let name = parts.next().unwrap_or("").trim().to_string();
            let kind = match parts.next().unwrap_or("").trim() {
                "struct" => DefKind::Struct,
                "enum"   => DefKind::Enum,
                "impl"   => DefKind::Impl,
                "trait"  => DefKind::Trait,
                "mod"    => DefKind::Mod,
                _        => DefKind::Fn,
            };
            (name, kind)
        } else {
            (inner.trim().to_string(), DefKind::Fn)
        }
    } else {
        ("unnamed".into(), DefKind::Fn)
    }
}

fn collect_body(tokens: &[Token], pos: &mut usize) -> Vec<Node> {
    // Body = sequence of nodes on the same "line" until Newline or end
    let mut body = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Newline => { *pos += 1; break; }
            Token::Glyph('κ') => break, // new definition starts new scope
            _ => {
                if let Some(n) = parse_one(tokens, pos) {
                    body.push(n);
                } else {
                    break;
                }
            }
        }
    }
    body
}
