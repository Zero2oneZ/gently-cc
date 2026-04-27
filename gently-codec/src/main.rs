mod crystal;
mod grid;
mod parser;
mod pin;
mod scope;

use clap::{Parser, Subcommand};
use crystal::{list_crystals, load_crystal, dump_scoped};
use grid::EdgeKind;
use parser::{compress_scoped, expand_scoped};
use scope::{project_id_from_cwd, session_id, user_id, ScopeLevel, ScopedGrid};

#[derive(Parser)]
#[command(
    name = "codec",
    about = "gently-codec — hierarchical pin grid: GLOBAL→PROJECT→USER→CHAT"
)]
struct Cli {
    /// Scope for define/edge/promote operations (global/project/user/chat)
    #[arg(long, global = true, default_value = "chat")]
    scope: String,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Define a pin at --scope level
    Define { name: String, label: Vec<String> },
    /// Show what a pin resolves to (walks the full tree)
    Ref { name: String },
    /// Add directed edge between two pins
    Edge { from: String, to: String },
    /// Merge two pins — same referent [*a ≡ *b]
    Merge { a: String, b: String },
    /// Tombstone a pin — deprecated, address stays valid
    Drop { name: String },
    /// Show pin table — all scopes, resolution order
    Table,
    /// Compress text — parse inline [*pin: label] syntax
    Compress { text: Vec<String> },
    /// Expand *pin refs to full labels
    Expand { text: Vec<String> },
    /// Emit the project-level context dump (for model injection / onboarding)
    Context,
    /// Promote a pin from one scope to the next level up
    Promote { name: String, #[arg(long)] to: Option<String> },
    /// Show promotion candidates
    Suggest,
    /// Seal current session as Crystal
    Crystal,
    /// Load a Crystal by id prefix and restore its chat grid
    Load { id: String },
    /// List saved Crystals
    Crystals,
    /// Hook mode: stdin JSON → stdout JSON (pipeline integration)
    Hook,
    /// Stats across all scopes
    Stats,
    /// Show the conflict table — same name at different scopes
    Conflicts,
}

fn parse_scope(s: &str) -> ScopeLevel {
    match s.to_lowercase().as_str() {
        "global"  | "g" => ScopeLevel::Global,
        "project" | "p" => ScopeLevel::Project,
        "user"    | "u" => ScopeLevel::User,
        _               => ScopeLevel::Chat,
    }
}

fn load_scoped() -> ScopedGrid {
    ScopedGrid::load(&project_id_from_cwd(), &user_id(), &session_id())
}

fn main() {
    let cli = Cli::parse();
    let scope = parse_scope(&cli.scope);

    match cli.cmd {
        Cmd::Define { name, label } => cmd_define(scope, &name, &label.join(" ")),
        Cmd::Ref { name }           => cmd_ref(&name),
        Cmd::Edge { from, to }      => cmd_edge(scope, &from, &to),
        Cmd::Merge { a, b }         => cmd_merge(scope, &a, &b),
        Cmd::Drop { name }          => cmd_drop(&name),
        Cmd::Table                  => cmd_table(),
        Cmd::Compress { text }      => cmd_compress(&text.join(" ")),
        Cmd::Expand { text }        => cmd_expand(&text.join(" ")),
        Cmd::Context                => cmd_context(),
        Cmd::Promote { name, to }   => cmd_promote(&name, to.as_deref()),
        Cmd::Suggest                => cmd_suggest(),
        Cmd::Crystal                => cmd_crystal(),
        Cmd::Load { id }            => cmd_load(&id),
        Cmd::Crystals               => cmd_crystals(),
        Cmd::Hook                   => cmd_hook(),
        Cmd::Stats                  => cmd_stats(),
        Cmd::Conflicts              => cmd_conflicts(),
    }
}

fn cmd_define(scope: ScopeLevel, name: &str, label: &str) {
    let mut sg = load_scoped();
    let is_new = sg.define_at(scope, name, label);
    sg.save_all();
    if is_new {
        let tks = (label.chars().count() / 4).max(1).saturating_sub(1);
        println!("[{}] *{} = \"{}\"  ({} tokens/use)", scope.name(), name, label, tks);
    } else {
        // Check if it already exists at a different level
        if let Some(r) = sg.resolve(name) {
            if r.level != scope {
                println!("*{} already defined at [{}] — overriding at [{}] would shadow it", name, r.level.name(), scope.name());
                println!("  use --scope {} to redefine there, or choose a different name", r.level.name());
            } else {
                println!("*{} already defined at [{}]", name, scope.name());
            }
        }
    }
}

fn cmd_ref(name: &str) {
    let sg = load_scoped();
    match sg.resolve(name) {
        Some(r) => println!("[{}] *{} = \"{}\"", r.level.name(), name, r.pin.label),
        None    => eprintln!("*{} not found in any scope", name),
    }
}

fn cmd_edge(scope: ScopeLevel, from: &str, to: &str) {
    let mut sg = load_scoped();
    sg.add_edge(from, to, EdgeKind::Directed);
    sg.save_all();
    println!("*{} → *{}", from, to);
}

fn cmd_merge(scope: ScopeLevel, a: &str, b: &str) {
    let mut sg = load_scoped();
    sg.add_edge(a, b, EdgeKind::Merge);
    sg.save_all();
    println!("*{} ≡ *{}  (same referent)", a, b);
}

fn cmd_drop(name: &str) {
    let mut sg = load_scoped();
    // Tombstone at whichever level it lives
    let level = sg.resolve(name).map(|r| r.level);
    let tombstoned = match level {
        Some(l) => { let g = sg.grid_mut_pub(l); g.deprecate(name) }
        None    => false,
    };
    sg.save_all();
    if tombstoned {
        println!("†*{}  — tombstoned at [{}]. Address stays valid.", name, level.unwrap().name());
    } else {
        eprintln!("*{} not found", name);
    }
}

fn cmd_table() {
    let sg = load_scoped();

    let scopes = [
        (ScopeLevel::Global,  &sg.global),
        (ScopeLevel::Project, &sg.project),
        (ScopeLevel::User,    &sg.user),
        (ScopeLevel::Chat,    &sg.chat),
    ];

    let any = scopes.iter().any(|(_, g)| !g.pins.is_empty());
    if !any {
        println!("all scopes empty — define pins with: codec define <name> <label>");
        return;
    }

    for (level, grid) in &scopes {
        if grid.pins.is_empty() { continue; }
        println!("[{}]", level.name());
        for pin in grid.sorted_pins() {
            let tomb = if pin.tombstone { "†" } else { " " };
            println!("  {}{:<14} {:>4} refs  {}", tomb, pin.name, pin.ref_count, pin.label);
        }
        for e in &grid.edges {
            let op = match e.kind { EdgeKind::Directed => "→", EdgeKind::Merge => "≡" };
            println!("    *{} {} *{}", e.from, op, e.to);
        }
    }

    println!();
    println!("  {} tokens saved (all scopes)", sg.total_saved());
    println!("  project: {}  user: {}  session: {}", sg.project_id, sg.user_id, sg.session_id);
}

fn cmd_compress(text: &str) {
    let mut sg = load_scoped();
    sg.chat.advance_turn();
    let result = compress_scoped(text, &mut sg);
    sg.save_all();
    println!("{}", result.output);
    if !result.pins_defined.is_empty() {
        let names: Vec<&str> = result.pins_defined.iter().map(|(n,_)| n.as_str()).collect();
        eprintln!("📌 defined: {}", names.join(", "));
    }
    if !result.pins_referenced.is_empty() {
        eprintln!("⚡ refs: {}", result.pins_referenced.join(", "));
    }
}

fn cmd_expand(text: &str) {
    let sg = load_scoped();
    println!("{}", expand_scoped(text, &sg));
}

fn cmd_context() {
    let sg = load_scoped();
    print!("{}", sg.context_dump());
}

fn cmd_promote(name: &str, to_str: Option<&str>) {
    let mut sg = load_scoped();
    let from_level = match sg.resolve(name) {
        Some(r) => r.level,
        None    => { eprintln!("*{} not found", name); return; }
    };

    let to_level = if let Some(s) = to_str {
        parse_scope(s)
    } else {
        // Auto: one level up
        match from_level {
            ScopeLevel::Chat    => ScopeLevel::User,
            ScopeLevel::User    => ScopeLevel::Project,
            ScopeLevel::Project => ScopeLevel::Global,
            ScopeLevel::Global  => { println!("*{} already at global scope", name); return; }
        }
    };

    if to_level >= from_level {
        eprintln!("can only promote upward (lower level number = higher scope)");
        return;
    }

    if sg.promote(name, from_level, to_level) {
        sg.save_all();
        println!("promoted *{} from [{}] → [{}]", name, from_level.name(), to_level.name());
    } else {
        eprintln!("*{} already exists at [{}]", name, to_level.name());
    }
}

fn cmd_suggest() {
    let sg = load_scoped();
    let proposals = sg.promotion_candidates();
    if proposals.is_empty() {
        println!("no promotion candidates yet");
        return;
    }
    println!("Promotion proposals:");
    println!("{:<14} {:<10} {:<10} evidence", "pin", "from", "to");
    println!("{}", "-".repeat(60));
    for p in &proposals {
        println!("  *{:<13} {:<10} {:<10} {}", p.name, p.from_level.name(), p.to_level.name(), p.evidence);
        println!("    label: {}", p.label);
        println!("    accept: codec promote {} --to {}", p.name, p.to_level.name());
    }
}

fn cmd_crystal() {
    let sg = load_scoped();
    let crystal = dump_scoped(&sg);
    println!("{}", crystal.id);
    println!("  project:{} user:{} session:{}", sg.project_id, sg.user_id, sg.session_id);
    println!("  pins — global:{} project:{} user:{} chat:{}",
        sg.global.pins.len(), sg.project.pins.len(), sg.user.pins.len(), sg.chat.pins.len());
    println!("  {} tokens saved", crystal.tokens_saved);
}

fn cmd_load(id_prefix: &str) {
    match load_crystal(id_prefix) {
        Some(crystal) => {
            // Restore chat-level grid into current session
            let proj = project_id_from_cwd();
            let usr  = user_id();
            let sess = session_id();
            scope::save_scope_grid(&crystal.grid, ScopeLevel::Chat, &proj, &usr, &sess);
            println!("loaded Crystal {}  → chat grid restored", &crystal.id[..38]);
            println!("  {} pins · {} edges", crystal.grid.pins.len(), crystal.grid.edges.len());
        }
        None => eprintln!("no crystal matching '{}'", id_prefix),
    }
}

fn cmd_crystals() {
    let crystals = list_crystals();
    if crystals.is_empty() { println!("no crystals yet"); return; }
    println!("{:<20} {:<8} {:<8} session", "id", "pins", "saved");
    println!("{}", "-".repeat(60));
    for c in &crystals {
        println!("{:<20} {:<8} {:<8} {}",
            &c.id[7..23], c.grid.pins.len(), c.tokens_saved, c.session_id);
    }
}

fn cmd_hook() {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap_or(0);

    let val: serde_json::Value = match serde_json::from_str(input.trim()) {
        Ok(v) => v,
        Err(_) => { print!("{}", serde_json::json!({"continue": true})); return; }
    };

    let prompt = match val.get("prompt").and_then(|p| p.as_str()) {
        Some(p) => p.to_string(),
        None    => { print!("{}", serde_json::json!({"continue": true})); return; }
    };

    let mut sg = load_scoped();
    sg.chat.advance_turn();
    let turn = sg.chat.turn;

    let result = compress_scoped(&prompt, &mut sg);
    sg.save_all();

    let total = sg.total_saved();

    if !result.pins_defined.is_empty() || !result.pins_referenced.is_empty() {
        eprintln!("📌 CODEC +defined:{} refs:{} saved:{}",
            result.pins_defined.len(), result.pins_referenced.len(), total);
    }

    let proposals = sg.promotion_candidates();
    if !proposals.is_empty() {
        let names: Vec<&str> = proposals.iter().map(|p| p.name.as_str()).collect();
        eprintln!("~ promote candidates: {}", names.join(", "));
    }

    // Emit wiring data for the JS hook to feed back into BARF
    // Wire 1: new pin labels + ALL referenced labels (seed-on-reference promotes winding)
    let new_labels: Vec<&str> = result.pins_defined.iter().map(|(_, l)| l.as_str()).collect();
    let ref_labels: Vec<String> = result.pins_referenced.iter()
        .filter_map(|name| sg.resolve(name).map(|r| r.pin.label.clone()))
        .collect();
    // Wire 2: co-occurrence pairs by label (BARF works by blake3(label))
    let cooccur_labels: Vec<[String; 2]> = result.cooccur_pairs.iter().filter_map(|(a, b)| {
        let la = sg.resolve(a).map(|r| r.pin.label.clone())?;
        let lb = sg.resolve(b).map(|r| r.pin.label.clone())?;
        Some([la, lb])
    }).collect();
    // Wire 3: agency observations (pin_name, frame_char)
    let agency: Vec<serde_json::Value> = result.agency_observations.iter().filter_map(|(name, frame)| {
        sg.resolve(name).map(|r| serde_json::json!({
            "label": r.pin.label,
            "frame": frame.to_string()
        }))
    }).collect();

    print!("{}", serde_json::json!({
        "continue": true,
        "prompt": result.output,
        "turn": turn,
        "new_labels": new_labels,
        "ref_labels": ref_labels,
        "cooccur_labels": cooccur_labels,
        "agency": agency,
    }));
}

fn cmd_stats() {
    let sg = load_scoped();
    println!("gently-codec · Scoped Pin Grid");
    println!("  project_id : {}", sg.project_id);
    println!("  user_id    : {}", sg.user_id);
    println!("  session    : {}", sg.session_id);
    println!("  global pins: {}", sg.global.pins.len());
    println!("  project    : {}", sg.project.pins.len());
    println!("  user       : {}", sg.user.pins.len());
    println!("  chat       : {} (turn {})", sg.chat.pins.len(), sg.chat.turn);
    println!("  total saved: {} tokens", sg.total_saved());
}

fn cmd_conflicts() {
    let sg = load_scoped();
    let mut seen: std::collections::HashMap<&str, Vec<ScopeLevel>> = std::collections::HashMap::new();

    let scopes = [
        (ScopeLevel::Global,  &sg.global),
        (ScopeLevel::Project, &sg.project),
        (ScopeLevel::User,    &sg.user),
        (ScopeLevel::Chat,    &sg.chat),
    ];

    for (level, grid) in &scopes {
        for name in grid.pins.keys() {
            seen.entry(name.as_str()).or_default().push(*level);
        }
    }

    let conflicts: Vec<_> = seen.iter().filter(|(_, v)| v.len() > 1).collect();
    if conflicts.is_empty() {
        println!("no conflicts — all pins have unique names across scopes");
        return;
    }

    println!("Conflicts (same name at multiple scopes — chat shadows higher):");
    println!("{:<16} {:<30} scopes", "name", "chat value");
    println!("{}", "-".repeat(70));
    for (name, levels) in conflicts {
        let chat_label = sg.chat.pins.get(*name).map(|p| p.label.as_str()).unwrap_or("-");
        let scope_names: Vec<&str> = levels.iter().map(|l| l.name()).collect();
        println!("  *{:<14} {:<30} {}", name, &chat_label[..chat_label.len().min(28)], scope_names.join(" shadows "));
        for (level, grid) in &scopes {
            if let Some(pin) = grid.pins.get(*name) {
                println!("    [{}] = \"{}\"", level.name(), pin.label);
            }
        }
    }
}
