// CODIE DAG — every node variant is one CODIE primitive.
// Fractal property: any node can contain any node.
// The tree IS the program. The emitter just walks it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty:   Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Node {
    // ── κ  define function / struct / impl ──────────────────────
    Define {
        name:   String,
        kind:   DefKind,
        params: Vec<Param>,
        body:   Vec<Node>,
    },

    // ── β  fetch / get / call ───────────────────────────────────
    Fetch {
        source: String,           // "db/users", "http/api", fn name
        key:    Option<Box<Node>>,
    },

    // ── ε  bind / let ───────────────────────────────────────────
    Bind {
        name:  String,
        ty:    Option<String>,
        value: Option<Box<Node>>,
    },

    // ── ⁇  conditional ──────────────────────────────────────────
    Cond {
        pred:  Box<Node>,
        then:  Box<Node>,
        else_: Option<Box<Node>>,
    },

    // ── →  return ───────────────────────────────────────────────
    Return { value: Box<Node> },

    // ── ∀  loop / iterator ──────────────────────────────────────
    Loop {
        iter: Box<Node>,
        body: Vec<Node>,
    },

    // ── ⊥  error / fail ─────────────────────────────────────────
    Fail { reason: String },

    // ── ⊤  success / ok ─────────────────────────────────────────
    Ok { value: Box<Node> },

    // ── ∷  compose — pipe A into B ──────────────────────────────
    Pipe {
        steps: Vec<Node>,
    },

    // ── ⟹  flow with label ──────────────────────────────────────
    Flow {
        from:  Box<Node>,
        to:    Box<Node>,
        label: Option<String>,
    },

    // ── ¬  not / negate ─────────────────────────────────────────
    Not { inner: Box<Node> },

    // ── Atom  literal / identifier / type name ───────────────────
    Atom { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefKind {
    Fn,
    Struct,
    Enum,
    Impl,
    Trait,
    Mod,
}

impl Node {
    /// The fractal walk — apply f to every node recursively, same call at every depth.
    pub fn walk<F: FnMut(&Node)>(&self, f: &mut F) {
        f(self);
        match self {
            Node::Define { body, params, .. } => {
                for p in params { if let Some(ref _n) = p.ty {} }
                for n in body { n.walk(f); }
            }
            Node::Fetch  { key, .. }          => { if let Some(k) = key { k.walk(f); } }
            Node::Bind   { value, .. }         => { if let Some(v) = value { v.walk(f); } }
            Node::Cond   { pred, then, else_ } => {
                pred.walk(f); then.walk(f);
                if let Some(e) = else_ { e.walk(f); }
            }
            Node::Return { value }             => value.walk(f),
            Node::Loop   { iter, body }        => { iter.walk(f); for n in body { n.walk(f); } }
            Node::Fail   { .. }                => {}
            Node::Ok     { value }             => value.walk(f),
            Node::Pipe   { steps }             => { for n in steps { n.walk(f); } }
            Node::Flow   { from, to, .. }      => { from.walk(f); to.walk(f); }
            Node::Not    { inner }             => inner.walk(f),
            Node::Atom   { .. }                => {}
        }
    }

    /// Hash the DAG node — same algorithm the FCT uses. Leaf = hash(value), branch = hash(children).
    pub fn dag_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        format!("{:?}", self).hash(&mut h);
        format!("{:016x}", h.finish())
    }
}
