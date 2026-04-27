mod barf;
mod foam;
mod torus;
#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};
use foam::{load_foam, save_foam};
use torus::AgencyFrame;

#[derive(Parser)]
#[command(name = "barf", about = "BS-Artisan BARF foam retrieval")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Query foam for relevant tori (weight-driven scoring)
    Query {
        text: String,
        #[arg(short, long, default_value = "5")]
        max: usize,
    },
    /// Insert a new torus (or promote existing) into foam
    Insert {
        label: String,
        #[arg(short, long, default_value = "10")]
        tokens: usize,
    },
    /// Wire 2: record co-occurrence between two tori by label
    Cooccur { label_a: String, label_b: String },
    /// Wire 3: record agency frame for a torus by label
    Frame {
        label: String,
        /// frame: query/command/return/assert/ground/fuzzy/define
        frame: String,
    },
    /// Blend two tori (by hex id prefix)
    Blend {
        a: String,
        b: String,
        #[arg(short, long, default_value = "0.5")]
        strength: f64,
    },
    /// List all tori sorted by weight
    List,
    /// Foam stats
    Stats,
    /// Hook mode: stdin JSON → stdout JSON with semantic context injection
    Hook,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Query { text, max }          => cmd_query(&text, max),
        Cmd::Insert { label, tokens }     => cmd_insert(&label, tokens),
        Cmd::Cooccur { label_a, label_b } => cmd_cooccur(&label_a, &label_b),
        Cmd::Frame { label, frame }       => cmd_frame(&label, &frame),
        Cmd::Blend { a, b, strength }     => cmd_blend(&a, &b, strength),
        Cmd::List                         => cmd_list(),
        Cmd::Stats                        => cmd_stats(),
        Cmd::Hook                         => cmd_hook(),
    }
}

fn cmd_query(text: &str, max: usize) {
    let foam = load_foam();
    let results = foam.query(text, max);
    if results.is_empty() {
        println!("foam empty — no results");
        return;
    }
    println!("BARF  turn:{}  query: {}", foam.turn, text);
    println!("{:<8} {:<10} {:<8} {:<6} {:<8} label", "dist", "score", "weight", "wind", "cooccur");
    println!("{}", "-".repeat(72));
    for r in &results {
        println!(
            "{:<8.4} {:<10.4} {:<8.4} {:<6} {:<8} {}",
            r.distance,
            r.score,
            r.torus.weight(foam.turn),
            r.torus.winding,
            r.torus.cooccur.len(),
            r.torus.label
        );
    }
}

fn cmd_insert(label: &str, tokens: usize) {
    let mut foam = load_foam();
    let id = foam.insert(label, tokens);
    save_foam(&foam);
    let hex: String = id.iter().map(|b| format!("{:02x}", b)).collect();
    println!("inserted: {} → sha256:{}", label, &hex[..16]);
}

fn cmd_cooccur(label_a: &str, label_b: &str) {
    let mut foam = load_foam();
    foam.observe_cooccur(label_a, label_b);
    save_foam(&foam);
    // silent — called in bulk from hook
}

fn cmd_frame(label: &str, frame_str: &str) {
    let frame = parse_frame(frame_str);
    let mut foam = load_foam();
    foam.observe_frame(label, frame);
    save_foam(&foam);
}

fn parse_frame(s: &str) -> AgencyFrame {
    match s.to_lowercase().as_str() {
        "query"   | "?" => AgencyFrame::Query,
        "command" | ">" => AgencyFrame::Command,
        "return"  | "<" => AgencyFrame::Return,
        "assert"  | "!" => AgencyFrame::Assert,
        "ground"  | "=" => AgencyFrame::Ground,
        "fuzzy"   | "~" => AgencyFrame::Fuzzy,
        "define"  | ":" => AgencyFrame::Define,
        _               => AgencyFrame::None,
    }
}

fn cmd_blend(a_prefix: &str, b_prefix: &str, strength: f64) {
    let mut foam = load_foam();
    let a_hex = resolve_prefix(&foam, a_prefix);
    let b_hex = resolve_prefix(&foam, b_prefix);
    match (a_hex, b_hex) {
        (Some(a), Some(b)) => {
            foam.blend(&a, &b, strength);
            save_foam(&foam);
            println!("blended {} ↔ {} @ {:.2}", &a[..8], &b[..8], strength);
        }
        (None, _) => eprintln!("error: no torus matching '{}'", a_prefix),
        (_, None) => eprintln!("error: no torus matching '{}'", b_prefix),
    }
}

fn resolve_prefix(foam: &foam::Foam, prefix: &str) -> Option<String> {
    foam.tori.keys().find(|k| k.starts_with(prefix)).cloned()
}

fn cmd_list() {
    let foam = load_foam();
    if foam.tori.is_empty() { println!("foam is empty"); return; }
    println!("turn: {}  tori: {}", foam.turn, foam.tori.len());
    println!("{:<18} {:<8} {:<8} {:<8} label", "id", "weight", "wind", "cooccur");
    println!("{}", "-".repeat(70));
    let mut entries: Vec<_> = foam.tori.values().collect();
    entries.sort_by(|a, b| b.weight(foam.turn).partial_cmp(&a.weight(foam.turn)).unwrap());
    for t in entries {
        println!("{:<18} {:<8.3} {:<8} {:<8} {}",
            &t.id_hex()[..16], t.weight(foam.turn), t.winding, t.cooccur.len(), t.label);
    }
}

fn cmd_stats() {
    let foam = load_foam();
    let s = foam.stats();
    println!("BS-Artisan Foam  turn:{}", foam.turn);
    println!("  tori    : {}", s.tori_count);
    println!("  blends  : {}", s.blend_count);
    println!("  avg trust: {:.4}", s.avg_trust);
    // Show top 3 by weight
    let mut entries: Vec<_> = foam.tori.values().collect();
    if !entries.is_empty() {
        entries.sort_by(|a, b| b.weight(foam.turn).partial_cmp(&a.weight(foam.turn)).unwrap());
        println!("  top slots:");
        for t in entries.iter().take(3) {
            println!("    {:.3} wt · {} cooccur · [{}] {}", t.weight(foam.turn), t.cooccur.len(), t.agency_bias as u8, t.label);
        }
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
        None => { print!("{}", serde_json::json!({"continue": true})); return; }
    };

    // Accept turn from upstream (codec passes it)
    let ext_turn = val.get("turn").and_then(|t| t.as_u64()).unwrap_or(0) as u32;

    let mut foam = load_foam();
    if ext_turn > foam.turn { foam.turn = ext_turn; }
    foam.advance_turn();

    let results = foam.query(&prompt, 3);
    save_foam(&foam);

    // Wire 4 active: score is now weight-driven, threshold is meaningful
    let context: Vec<&str> = results.iter()
        .filter(|r| r.score > 0.1)  // lower threshold now that weight is real
        .map(|r| r.torus.label.as_str())
        .collect();

    let enriched = if context.is_empty() {
        prompt.clone()
    } else {
        format!("[ctx:{}] {}", context.join(","), prompt)
    };

    if !context.is_empty() {
        eprintln!("🌀 BARF turn:{} · ctx: {}", foam.turn, context.join(", "));
    }

    print!("{}", serde_json::json!({"continue": true, "prompt": enriched}));
}
