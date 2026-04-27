// Pre-built DAG fragments. Call template("crud", "User") and get back a Node
// that expands into a complete CRUD module. Same fractal property: templates
// compose — template("api", "User") internally uses template("crud", "User").

use crate::ast::{DefKind, Node, Param};

pub fn expand(name: &str, subject: &str) -> Option<Node> {
    match name {
        "crud"   => Some(crud(subject)),
        "api"    => Some(api(subject)),
        "error"  => Some(error_enum(subject)),
        "repo"   => Some(repo_trait(subject)),
        "entity" => Some(entity_struct(subject)),
        _        => None,
    }
}

// ── entity — struct with standard fields ─────────────────────
fn entity_struct(name: &str) -> Node {
    Node::Define {
        name:   name.to_string(),
        kind:   DefKind::Struct,
        params: vec![
            Param { name: "id".into(),         ty: Some("uuid::Uuid".into()) },
            Param { name: "created_at".into(), ty: Some("chrono::DateTime<chrono::Utc>".into()) },
            Param { name: "updated_at".into(), ty: Some("chrono::DateTime<chrono::Utc>".into()) },
        ],
        body: vec![],
    }
}

// ── error — standard error enum ──────────────────────────────
fn error_enum(name: &str) -> Node {
    Node::Define {
        name: format!("{}Error", name),
        kind: DefKind::Enum,
        params: vec![],
        body: vec![
            Node::Atom { value: "NotFound".into() },
            Node::Atom { value: "Unauthorized".into() },
            Node::Atom { value: "Conflict".into() },
            Node::Atom { value: format!("Internal(String)") },
        ],
    }
}

// ── repo — repository trait ───────────────────────────────────
fn repo_trait(name: &str) -> Node {
    let lc = name.to_lowercase();
    Node::Define {
        name:   format!("{}Repository", name),
        kind:   DefKind::Trait,
        params: vec![],
        body:   vec![
            fn_sig(&format!("find_{}", lc),
                   vec![("id", "uuid::Uuid")],
                   &format!("Result<Option<{}>, {}Error>", name, name)),
            fn_sig(&format!("list_{}s", lc),
                   vec![],
                   &format!("Result<Vec<{}>, {}Error>", name, name)),
            fn_sig(&format!("create_{}", lc),
                   vec![("data", &format!("Create{}Request", name))],
                   &format!("Result<{}, {}Error>", name, name)),
            fn_sig(&format!("update_{}", lc),
                   vec![("id", "uuid::Uuid"), ("data", &format!("Update{}Request", name))],
                   &format!("Result<{}, {}Error>", name, name)),
            fn_sig(&format!("delete_{}", lc),
                   vec![("id", "uuid::Uuid")],
                   &format!("Result<(), {}Error>", name)),
        ],
    }
}

// ── crud — four functions: create / get / list / delete ──────
fn crud(name: &str) -> Node {
    let lc = name.to_lowercase();
    Node::Pipe {
        steps: vec![
            // create
            Node::Define {
                name:   format!("create_{}", lc),
                kind:   DefKind::Fn,
                params: vec![
                    Param { name: "db".into(),   ty: Some("&sqlx::PgPool".into()) },
                    Param { name: "data".into(), ty: Some(format!("Create{}Request", name)) },
                ],
                body: vec![
                    Node::Fetch { source: format!("db/{}.insert", lc), key: Some(Box::new(Node::Atom { value: "data".into() })) },
                    Node::Ok    { value: Box::new(Node::Atom { value: name.into() }) },
                ],
            },
            // get
            Node::Define {
                name:   format!("get_{}", lc),
                kind:   DefKind::Fn,
                params: vec![
                    Param { name: "db".into(), ty: Some("&sqlx::PgPool".into()) },
                    Param { name: "id".into(), ty: Some("uuid::Uuid".into()) },
                ],
                body: vec![
                    Node::Bind {
                        name:  "row".into(),
                        ty:    None,
                        value: Some(Box::new(Node::Fetch {
                            source: format!("db/{}.find", lc),
                            key:    Some(Box::new(Node::Atom { value: "id".into() })),
                        })),
                    },
                    Node::Cond {
                        pred:  Box::new(Node::Not { inner: Box::new(Node::Atom { value: "row.is_some()".into() }) }),
                        then:  Box::new(Node::Fail { reason: format!("{}Error::NotFound", name) }),
                        else_: None,
                    },
                    Node::Ok { value: Box::new(Node::Atom { value: "row.unwrap()".into() }) },
                ],
            },
            // list
            Node::Define {
                name:   format!("list_{}s", lc),
                kind:   DefKind::Fn,
                params: vec![
                    Param { name: "db".into(), ty: Some("&sqlx::PgPool".into()) },
                ],
                body: vec![
                    Node::Fetch { source: format!("db/{}.list", lc), key: None },
                    Node::Ok    { value: Box::new(Node::Atom { value: "rows".into() }) },
                ],
            },
            // delete
            Node::Define {
                name:   format!("delete_{}", lc),
                kind:   DefKind::Fn,
                params: vec![
                    Param { name: "db".into(), ty: Some("&sqlx::PgPool".into()) },
                    Param { name: "id".into(), ty: Some("uuid::Uuid".into()) },
                ],
                body: vec![
                    Node::Fetch { source: format!("db/{}.delete", lc), key: Some(Box::new(Node::Atom { value: "id".into() })) },
                    Node::Ok    { value: Box::new(Node::Atom { value: "()".into() }) },
                ],
            },
        ],
    }
}

// ── api — crud + HTTP handler wrappers ───────────────────────
fn api(name: &str) -> Node {
    let lc = name.to_lowercase();
    // Compose: entity struct + error enum + crud functions + axum handlers
    Node::Pipe {
        steps: vec![
            entity_struct(name),
            error_enum(name),
            crud(name),                          // reuse crud template
            axum_handlers(name, &lc),
        ],
    }
}

fn axum_handlers(name: &str, lc: &str) -> Node {
    Node::Define {
        name:   format!("{}_routes", lc),
        kind:   DefKind::Mod,
        params: vec![],
        body:   vec![
            Node::Define {
                name:   format!("get_{}", lc),
                kind:   DefKind::Fn,
                params: vec![
                    Param { name: "State(db)".into(), ty: Some("State<sqlx::PgPool>".into()) },
                    Param { name: "Path(id)".into(),  ty: Some("Path<uuid::Uuid>".into()) },
                ],
                body: vec![
                    Node::Bind {
                        name:  "result".into(),
                        ty:    None,
                        value: Some(Box::new(Node::Fetch {
                            source: format!("super::get_{}", lc),
                            key:    Some(Box::new(Node::Atom { value: "&db, id".into() })),
                        })),
                    },
                    Node::Cond {
                        pred:  Box::new(Node::Atom { value: "result.is_err()".into() }),
                        then:  Box::new(Node::Return {
                            value: Box::new(Node::Atom { value: "(StatusCode::NOT_FOUND, Json(serde_json::json!({\"error\": \"not found\"}))).into_response()".into() })
                        }),
                        else_: None,
                    },
                    Node::Return {
                        value: Box::new(Node::Atom { value: "Json(result.unwrap()).into_response()".into() })
                    },
                ],
            },
        ],
    }
}

// helper — trait method signature (no body)
fn fn_sig(name: &str, params: Vec<(&str, &str)>, ret: &str) -> Node {
    Node::Atom {
        value: format!(
            "async fn {}({}) -> {};",
            name,
            params.iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<_>>().join(", "),
            ret
        ),
    }
}
