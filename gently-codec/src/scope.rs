//! Hierarchical slot namespace: GLOBAL → PROJECT → USER → CHAT
//!
//! Resolution: walk up the tree, first hit wins.
//! Lower scopes override higher scopes.
//! Higher scopes are visible to all children below.
//!
//! Promotion:
//!   chat pin ref_count >= 5            → candidate for USER
//!   user pin seen in 3+ sessions       → candidate for PROJECT
//!   project pin seen in 2+ projects    → candidate for GLOBAL (human-gated)

use crate::grid::{EdgeKind, PinGrid};
use crate::pin::Pin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScopeLevel {
    Global  = 0,
    Project = 1,
    User    = 2,
    Chat    = 3,
}

impl ScopeLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Global  => "global",
            Self::Project => "project",
            Self::User    => "user",
            Self::Chat    => "chat",
        }
    }
}

#[derive(Debug)]
pub struct ResolvedPin<'a> {
    pub pin: &'a Pin,
    pub level: ScopeLevel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromotionProposal {
    pub name: String,
    pub label: String,
    pub from_level: ScopeLevel,
    pub to_level: ScopeLevel,
    pub evidence: String,  // why this was proposed
}

/// The full scoped namespace.
pub struct ScopedGrid {
    pub global:  PinGrid,
    pub project: PinGrid,
    pub user:    PinGrid,
    pub chat:    PinGrid,
    pub project_id: String,
    pub user_id: String,
    pub session_id: String,
}

impl ScopedGrid {
    pub fn load(project_id: &str, user_id: &str, session_id: &str) -> Self {
        ScopedGrid {
            global:     load_scope_grid(ScopeLevel::Global,  project_id, user_id, session_id),
            project:    load_scope_grid(ScopeLevel::Project, project_id, user_id, session_id),
            user:       load_scope_grid(ScopeLevel::User,    project_id, user_id, session_id),
            chat:       load_scope_grid(ScopeLevel::Chat,    project_id, user_id, session_id),
            project_id: project_id.to_string(),
            user_id:    user_id.to_string(),
            session_id: session_id.to_string(),
        }
    }

    pub fn save_chat(&self) {
        save_scope_grid(&self.chat, ScopeLevel::Chat, &self.project_id, &self.user_id, &self.session_id);
    }

    pub fn save_all(&self) {
        save_scope_grid(&self.global,  ScopeLevel::Global,  &self.project_id, &self.user_id, &self.session_id);
        save_scope_grid(&self.project, ScopeLevel::Project, &self.project_id, &self.user_id, &self.session_id);
        save_scope_grid(&self.user,    ScopeLevel::User,    &self.project_id, &self.user_id, &self.session_id);
        save_scope_grid(&self.chat,    ScopeLevel::Chat,    &self.project_id, &self.user_id, &self.session_id);
    }

    /// Walk up the tree: chat → user → project → global.
    /// First hit wins. Lower scopes override higher.
    pub fn resolve(&self, name: &str) -> Option<ResolvedPin<'_>> {
        let levels = [
            (ScopeLevel::Chat,    &self.chat),
            (ScopeLevel::User,    &self.user),
            (ScopeLevel::Project, &self.project),
            (ScopeLevel::Global,  &self.global),
        ];
        for (level, grid) in &levels {
            if let Some(pin) = grid.pins.get(name) {
                if !pin.tombstone {
                    return Some(ResolvedPin { pin, level: *level });
                }
            }
        }
        None
    }

    /// Define a pin at the given scope level.
    pub fn define_at(&mut self, level: ScopeLevel, name: &str, label: &str) -> bool {
        let grid = self.grid_mut(level);
        grid.define(name, label)
    }

    /// Reference a pin — finds it at the highest (most specific) scope, increments count.
    pub fn reference(&mut self, name: &str) -> Option<(String, ScopeLevel)> {
        // Find the level first
        let level = self.resolve(name)?.level;
        let label = self.grid_mut(level).reference(name)?;
        Some((label, level))
    }

    /// Add edge at the lowest scope that contains both pins.
    pub fn add_edge(&mut self, from: &str, to: &str, kind: EdgeKind) {
        // Find which level to put the edge at
        let from_level = self.resolve(from).map(|r| r.level);
        let to_level   = self.resolve(to).map(|r| r.level);
        if let (Some(fl), Some(tl)) = (from_level, to_level) {
            // Edge goes at the lower (more specific) level
            let target = fl.max(tl);
            self.grid_mut(target).add_edge(from, to, kind);
        }
    }

    fn grid_mut(&mut self, level: ScopeLevel) -> &mut PinGrid {
        match level {
            ScopeLevel::Global  => &mut self.global,
            ScopeLevel::Project => &mut self.project,
            ScopeLevel::User    => &mut self.user,
            ScopeLevel::Chat    => &mut self.chat,
        }
    }

    pub fn grid_mut_pub(&mut self, level: ScopeLevel) -> &mut PinGrid {
        self.grid_mut(level)
    }

    /// Scan for promotion candidates.
    pub fn promotion_candidates(&self) -> Vec<PromotionProposal> {
        let mut proposals = Vec::new();

        // Chat pins with ref_count >= 5 → suggest user promotion
        for pin in self.chat.pins.values() {
            if pin.tombstone || pin.ref_count < 5 { continue; }
            // Don't propose if already exists at user or above
            if self.resolve_above(ScopeLevel::User, &pin.name).is_some() { continue; }
            proposals.push(PromotionProposal {
                name: pin.name.clone(),
                label: pin.label.clone(),
                from_level: ScopeLevel::Chat,
                to_level: ScopeLevel::User,
                evidence: format!("{} refs in this chat", pin.ref_count),
            });
        }

        // User pins with high ref_count → suggest project promotion
        for pin in self.user.pins.values() {
            if pin.tombstone || pin.ref_count < 3 { continue; }
            if self.resolve_above(ScopeLevel::Project, &pin.name).is_some() { continue; }
            proposals.push(PromotionProposal {
                name: pin.name.clone(),
                label: pin.label.clone(),
                from_level: ScopeLevel::User,
                to_level: ScopeLevel::Project,
                evidence: format!("{} total refs by this user", pin.ref_count),
            });
        }

        proposals
    }

    fn resolve_above(&self, at_or_above: ScopeLevel, name: &str) -> Option<ResolvedPin<'_>> {
        let levels: &[(ScopeLevel, &PinGrid)] = match at_or_above {
            ScopeLevel::User    => &[(ScopeLevel::User, &self.user), (ScopeLevel::Project, &self.project), (ScopeLevel::Global, &self.global)],
            ScopeLevel::Project => &[(ScopeLevel::Project, &self.project), (ScopeLevel::Global, &self.global)],
            ScopeLevel::Global  => &[(ScopeLevel::Global, &self.global)],
            ScopeLevel::Chat    => return self.resolve(name),
        };
        for (level, grid) in levels {
            if let Some(pin) = grid.pins.get(name) {
                if !pin.tombstone {
                    return Some(ResolvedPin { pin, level: *level });
                }
            }
        }
        None
    }

    /// Execute a promotion proposal (after human acceptance).
    pub fn promote(&mut self, name: &str, from: ScopeLevel, to: ScopeLevel) -> bool {
        let label = {
            let grid = match from {
                ScopeLevel::Chat    => &self.chat,
                ScopeLevel::User    => &self.user,
                ScopeLevel::Project => &self.project,
                ScopeLevel::Global  => &self.global,
            };
            grid.pins.get(name).map(|p| p.label.clone())
        };
        if let Some(label) = label {
            self.define_at(to, name, &label)
        } else {
            false
        }
    }

    /// Total tokens saved across all scopes.
    pub fn total_saved(&self) -> usize {
        self.global.total_saved()
            + self.project.total_saved()
            + self.user.total_saved()
            + self.chat.total_saved()
    }

    /// Compact project-level context dump for model injection.
    /// "Drop this into context → model is instantly fluent in the project."
    pub fn context_dump(&self) -> String {
        let mut out = String::new();
        let project_pins = self.project.sorted_pins();
        let global_pins  = self.global.sorted_pins();

        if project_pins.is_empty() && global_pins.is_empty() {
            return "# no project-level pins defined yet\n".to_string();
        }

        out.push_str(&format!("# project:{} · {} pins\n", self.project_id, project_pins.len()));

        if !global_pins.is_empty() {
            out.push_str("## global\n");
            for p in &global_pins {
                out.push_str(&format!("*{} = {}\n", p.name, p.label));
            }
        }

        if !project_pins.is_empty() {
            out.push_str("## project\n");
            for p in &project_pins {
                out.push_str(&format!("*{} = {}\n", p.name, p.label));
            }
        }

        // Edges
        let edges: Vec<_> = self.project.edges.iter()
            .chain(self.global.edges.iter())
            .collect();
        if !edges.is_empty() {
            out.push_str("## edges\n");
            for e in edges {
                let op = match e.kind { EdgeKind::Directed => "→", EdgeKind::Merge => "≡" };
                out.push_str(&format!("*{} {} *{}\n", e.from, op, e.to));
            }
        }

        out
    }
}

// ── Storage paths ────────────────────────────────────────────────────────────

fn gently_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join(".gently")
}

fn scope_path(level: ScopeLevel, project_id: &str, user_id: &str, session_id: &str) -> PathBuf {
    match level {
        ScopeLevel::Global  => gently_home().join("global").join("pins.json"),
        ScopeLevel::Project => gently_home().join("projects").join(project_id).join("pins.json"),
        ScopeLevel::User    => gently_home().join("users").join(user_id).join("pins.json"),
        ScopeLevel::Chat    => gently_home().join("sessions").join(session_id).join("pins.json"),
    }
}

pub fn load_scope_grid(level: ScopeLevel, project_id: &str, user_id: &str, session_id: &str) -> PinGrid {
    let path = scope_path(level, project_id, user_id, session_id);
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_else(|_| PinGrid::new())
    } else {
        PinGrid::new()
    }
}

pub fn save_scope_grid(grid: &PinGrid, level: ScopeLevel, project_id: &str, user_id: &str, session_id: &str) {
    let path = scope_path(level, project_id, user_id, session_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(grid) {
        let _ = std::fs::write(path, data);
    }
}

/// Derive a project ID from the current working directory.
pub fn project_id_from_cwd() -> String {
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("/"))
        .to_string_lossy()
        .to_string();
    let hash = blake3::hash(cwd.as_bytes()).to_hex();
    hash[..16].to_string()
}

/// Derive user ID from env or ~/.gently/user.json.
pub fn user_id() -> String {
    if let Ok(id) = std::env::var("GENTLY_USER_ID") {
        return id;
    }
    let path = gently_home().join("user.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                return id.to_string();
            }
        }
    }
    // Default: hash of HOME path
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")).to_string_lossy().to_string();
    blake3::hash(home.as_bytes()).to_hex()[..12].to_string()
}

/// Session ID from env or a default.
pub fn session_id() -> String {
    std::env::var("CLAUDE_SESSION_ID")
        .unwrap_or_else(|_| "default".to_string())
}
