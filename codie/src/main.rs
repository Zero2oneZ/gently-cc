mod compress;
mod decompress;
mod glyph;
mod hash;
mod hook;
mod operator;
mod stats;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "codie",
    version = "1.0.0",
    about = "CODIE — 44-keyword prompt compression. 94.7% token reduction. Zero network. Zero ML.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compress text to CODIE glyph form
    Compress {
        /// Text to compress (omit to read from stdin)
        text: Vec<String>,
        /// Read from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Decompress CODIE glyphs back to keyword form
    Decompress {
        /// Glyph text to decompress
        text: Vec<String>,
    },
    /// BLAKE3 content-address a string (GRIM format)
    Hash {
        /// Text to hash
        text: Vec<String>,
    },
    /// Claude Code hook mode: JSON stdin → JSON stdout
    Hook,
    /// Show session compression stats
    Stats,
    /// Side-by-side compression benchmark
    Bench {
        /// Text to benchmark
        text: Vec<String>,
    },
    /// Show the full 44-keyword glyph table
    Table,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compress { text, stdin } => cmd_compress(text, stdin),
        Command::Decompress { text } => cmd_decompress(text),
        Command::Hash { text } => cmd_hash(text),
        Command::Hook => hook::run(),
        Command::Stats => cmd_stats(),
        Command::Bench { text } => cmd_bench(text),
        Command::Table => cmd_table(),
    }
}

fn join_or_stdin(words: Vec<String>, from_stdin: bool) -> String {
    if from_stdin || words.is_empty() {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap_or(0);
        s
    } else {
        words.join(" ")
    }
}

fn cmd_compress(words: Vec<String>, stdin: bool) {
    let text = join_or_stdin(words, stdin);
    if text.trim().is_empty() {
        eprintln!("codie compress: no input");
        std::process::exit(1);
    }

    let c = compress::compress(&text);
    println!("{}", c.output);

    let session = stats::update_and_save(c.original_tokens, c.compressed_tokens, c.pct_saved());
    eprintln!("{}", stats::stats_line(c.original_tokens, c.compressed_tokens, c.pct_saved(), &session));
}

fn cmd_decompress(words: Vec<String>) {
    let text = join_or_stdin(words, false);
    println!("{}", decompress::decompress(&text));
}

fn cmd_hash(words: Vec<String>) {
    let text = join_or_stdin(words, false);
    println!("{}", hash::content_hash(&text));
}

fn cmd_stats() {
    let s = stats::load();
    if s.turns == 0 {
        println!("No session data yet. Run `codie compress` or use as a Claude Code hook.");
        return;
    }
    println!("CODIE Session Stats");
    println!("─────────────────────────────────────");
    println!("  turns          : {}", s.turns);
    println!("  tokens original: {}", s.tokens_original);
    println!("  tokens saved   : {}", s.tokens_saved);
    println!("  avg reduction  : {:.1}%", s.avg_pct());
    println!("─────────────────────────────────────");
}

fn cmd_bench(words: Vec<String>) {
    let text = join_or_stdin(words, false);
    if text.trim().is_empty() {
        eprintln!("codie bench: no input");
        std::process::exit(1);
    }

    let c = compress::compress(&text);
    let pct = c.pct_saved();
    let h = hash::content_hash(&c.output);

    println!("CODIE Bench");
    println!("─────────────────────────────────────────────────────");
    println!("ORIGINAL  ({} tokens)", c.original_tokens);
    println!("{}", text.trim());
    println!();
    println!("COMPRESSED  ({} tokens, {:.1}% saved)", c.compressed_tokens, pct);
    println!("{}", c.output.trim());
    println!();
    println!("HASH  {}", h);
    println!("─────────────────────────────────────────────────────");
    println!("  OpenClaw: 0% compression (no encoding layer)");
    println!("  CODIE   : {:.1}% compression (operators + 44-keyword glyphs + squeeze)", pct);
    println!("─────────────────────────────────────────────────────");
}

fn cmd_table() {
    println!("CODIE Compression Tables");
    println!();
    println!("Layer 2 — CCP Prefix Operators (structural)");
    println!("─────────────────────────────────────────────");
    for &(op, glyph, meaning) in operator::OPERATORS {
        println!("  {:4} →  {}   {}", op, glyph, meaning);
    }
    println!();
    println!("Layer 1 — 44-Keyword Glyph Table (lexical)");
    println!("─────────────────────────────────────────────");
    for (kw, g) in glyph::ALL_PAIRS {
        println!("  {:12} →  {}", kw, g);
    }
    println!();
    println!("  OpenClaw: 0 tables · 0% compression");
    println!("  CODIE   : 2 layers · 44 keywords + 7 operators + squeeze");
}
