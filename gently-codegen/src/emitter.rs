// DAG Node → Rust source string.
// One emit() call per node variant. Recurse into children. That's the whole emitter.
// Fractal property: emit(node) calls emit(child) — same function at every depth.

use crate::ast::{DefKind, Node, Param};

pub struct Emitter {
    indent: usize,
}

impl Emitter {
    pub fn new() -> Self { Self { indent: 0 } }

    pub fn emit(&mut self, node: &Node) -> String {
        match node {
            Node::Define { name, kind, params, body } => self.emit_define(name, kind, params, body),
            Node::Fetch  { source, key }              => self.emit_fetch(source, key),
            Node::Bind   { name, ty, value }          => self.emit_bind(name, ty, value),
            Node::Cond   { pred, then, else_ }        => self.emit_cond(pred, then, else_),
            Node::Return { value }                    => match value.as_ref() {
                Node::Fail { .. } => self.emit(value),  // Fail already includes return
                _ => format!("{}return {};", self.pad(), self.emit(value)),
            },
            Node::Loop   { iter, body }               => self.emit_loop(iter, body),
            Node::Fail   { reason }                   => format!("{}return Err({}.into());", self.pad(), reason),
            Node::Ok     { value }                    => format!("Ok({})", self.emit(value)),
            Node::Pipe   { steps }                    => self.emit_pipe(steps),
            Node::Flow   { from, to, label }          => self.emit_flow(from, to, label),
            Node::Not    { inner }                    => format!("!{}", self.emit(inner)),
            Node::Atom   { value }                    => value.clone(),
        }
    }

    fn pad(&self) -> String { "    ".repeat(self.indent) }

    fn emit_define(&mut self, name: &str, kind: &DefKind, params: &[Param], body: &[Node]) -> String {
        match kind {
            DefKind::Struct => self.emit_struct(name, params),
            DefKind::Enum   => self.emit_enum(name, body),
            DefKind::Impl   => self.emit_impl(name, body),
            DefKind::Mod    => self.emit_mod(name, body),
            DefKind::Trait  => self.emit_trait(name, body),
            DefKind::Fn     => self.emit_fn(name, params, body),
        }
    }

    fn emit_fn(&mut self, name: &str, params: &[Param], body: &[Node]) -> String {
        let param_str = params.iter().map(|p| {
            format!("{}: {}", p.name, p.ty.as_deref().unwrap_or("_"))
        }).collect::<Vec<_>>().join(", ");

        // Infer return type from body: look for Ok/Fail nodes
        let ret = infer_return(body);

        let mut out = format!("{}pub fn {}({}) -> {} {{\n", self.pad(), name, param_str, ret);
        self.indent += 1;
        for n in body {
            let line = self.emit(n);
            if !line.trim().is_empty() {
                let padded = if line.starts_with(' ') || line.starts_with('\t') {
                    line
                } else {
                    format!("{}{}", self.pad(), line)
                };
                out.push_str(&padded);
                if !padded.trim_end().ends_with(';') && !padded.trim_end().ends_with('}') {
                    out.push(';');
                }
                out.push('\n');
            }
        }
        self.indent -= 1;
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_struct(&mut self, name: &str, params: &[Param]) -> String {
        let mut out = format!("{}#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n", self.pad());
        out.push_str(&format!("{}pub struct {} {{\n", self.pad(), name));
        for p in params {
            let ty = p.ty.as_deref().unwrap_or("String");
            out.push_str(&format!("{}    pub {}: {},\n", self.pad(), p.name, ty));
        }
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_enum(&mut self, name: &str, body: &[Node]) -> String {
        let mut out = format!("{}#[derive(Debug, Clone)]\n", self.pad());
        out.push_str(&format!("{}pub enum {} {{\n", self.pad(), name));
        for variant in body {
            if let Node::Atom { value } = variant {
                out.push_str(&format!("{}    {},\n", self.pad(), value));
            }
        }
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_impl(&mut self, name: &str, body: &[Node]) -> String {
        let mut out = format!("{}impl {} {{\n", self.pad(), name);
        self.indent += 1;
        for n in body { out.push_str(&self.emit(n)); out.push('\n'); }
        self.indent -= 1;
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_mod(&mut self, name: &str, body: &[Node]) -> String {
        let mut out = format!("{}pub mod {} {{\n", self.pad(), name);
        self.indent += 1;
        for n in body { out.push_str(&self.emit(n)); out.push('\n'); }
        self.indent -= 1;
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_trait(&mut self, name: &str, body: &[Node]) -> String {
        let mut out = format!("{}pub trait {} {{\n", self.pad(), name);
        self.indent += 1;
        for n in body { out.push_str(&self.emit(n)); }
        self.indent -= 1;
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_fetch(&mut self, source: &str, key: &Option<Box<Node>>) -> String {
        // Map CODIE source paths to Rust call patterns
        let (module, method) = split_source(source);
        match key {
            Some(k) => format!("{}::{}({}).await?", module, method, self.emit(k)),
            None    => format!("{}::{}().await?", module, method),
        }
    }

    fn emit_bind(&mut self, name: &str, ty: &Option<String>, value: &Option<Box<Node>>) -> String {
        match (ty, value) {
            (Some(t), Some(v)) => format!("{}let {}: {} = {};", self.pad(), name, t, self.emit(v)),
            (Some(t), None)    => format!("{}let {}: {};", self.pad(), name, t),
            (None,    Some(v)) => format!("{}let {} = {};", self.pad(), name, self.emit(v)),
            (None,    None)    => format!("{}let {};", self.pad(), name),
        }
    }

    fn emit_cond(&mut self, pred: &Node, then: &Node, else_: &Option<Box<Node>>) -> String {
        let pred_str = self.emit(pred);
        let mut out  = format!("{}if {} {{\n", self.pad(), pred_str);
        self.indent += 1;
        let then_str = self.emit(then);
        out.push_str(&then_str);
        if !then_str.trim_end().ends_with(';') && !then_str.trim_end().ends_with('}') {
            out.push(';');
        }
        out.push('\n');
        self.indent -= 1;
        out.push_str(&format!("{}}}", self.pad()));
        if let Some(e) = else_ {
            out.push_str(" else {\n");
            self.indent += 1;
            out.push_str(&self.emit(e));
            out.push('\n');
            self.indent -= 1;
            out.push_str(&format!("{}}}", self.pad()));
        }
        out
    }

    fn emit_loop(&mut self, iter: &Node, body: &[Node]) -> String {
        let iter_str = self.emit(iter);
        let mut out  = format!("{}for item in {} {{\n", self.pad(), iter_str);
        self.indent += 1;
        for n in body {
            let line = self.emit(n);
            out.push_str(&line);
            if !line.trim_end().ends_with(';') { out.push(';'); }
            out.push('\n');
        }
        self.indent -= 1;
        out.push_str(&format!("{}}}\n", self.pad()));
        out
    }

    fn emit_pipe(&mut self, steps: &[Node]) -> String {
        // Pipe = sequence of statements. Last one is the expression value.
        steps.iter().map(|n| {
            let s = self.emit(n);
            if s.trim_end().ends_with('}') { s } else { format!("{};", s) }
        }).collect::<Vec<_>>().join("\n")
    }

    fn emit_flow(&mut self, from: &Node, to: &Node, label: &Option<String>) -> String {
        let from_s = self.emit(from);
        let to_s   = self.emit(to);
        match label {
            Some(l) => format!("{}/* {} */ {}.pipe({})", self.pad(), l, from_s, to_s),
            None    => {
                // A→B where from is "_" means just emit `to` (parser placeholder)
                if from_s == "_" { to_s } else { format!("{}.and_then(|_| {{ {} }})", from_s, to_s) }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────

fn split_source(source: &str) -> (String, String) {
    // "db/users"      → ("db", "users::get")
    // "db/users.find" → ("db", "users::find")
    // "http/api"      → ("http_client", "api::call")
    // "bare_fn"       → ("self", "bare_fn")
    if let Some((ns, rest)) = source.split_once('/') {
        let (module, method) = rest.split_once('.').unwrap_or((rest, "get"));
        (ns.replace('-', "_"), format!("{}::{}", module, method))
    } else if let Some((module, method)) = source.split_once('.') {
        (module.to_string(), method.to_string())
    } else {
        ("self".into(), source.replace('-', "_"))
    }
}

fn infer_return(body: &[Node]) -> String {
    // Walk the body, look for Ok/Fail nodes to decide return type
    let mut has_ok   = false;
    let mut has_fail = false;
    let mut ok_type  = "()".to_string();

    for node in body {
        node.walk(&mut |n| match n {
            Node::Ok   { value } => { has_ok = true; ok_type = value_type(value); }
            Node::Fail { .. }    => { has_fail = true; }
            Node::Return { value } => { ok_type = value_type(value); }
            _ => {}
        });
    }

    if has_fail || has_ok {
        format!("Result<{}, Box<dyn std::error::Error>>", ok_type)
    } else {
        ok_type
    }
}

fn value_type(node: &Node) -> String {
    match node {
        Node::Atom { value } => {
            if value.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                value.clone()
            } else {
                "_".to_string()  // let rustc infer it
            }
        }
        Node::Fetch { source, .. } => {
            let (_, method) = split_source(source);
            let base = method.split("::").last().unwrap_or("Value");
            pascal_case(base)
        }
        _ => "_".into(),
    }
}

fn pascal_case(s: &str) -> String {
    s.split('_').map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    }).collect()
}
