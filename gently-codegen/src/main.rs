mod ast;
mod parser;
mod emitter;
mod templates;

use emitter::Emitter;
use std::io::Read;

fn usage() {
    eprintln!("gen — CODIE DAG → Rust source");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  gen '<codie expression>'");
    eprintln!("  gen --template <name> <Subject>");
    eprintln!("  gen --dag                        # read JSON DAG from stdin");
    eprintln!("  gen --dump '<expr>'              # print AST as JSON, don't emit");
    eprintln!();
    eprintln!("Templates:  crud  api  error  repo  entity");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  gen 'κ[auth] ε[uid:UserId] β[db/users←uid] ⁇¬found→⊥ →token'");
    eprintln!("  gen --template crud User");
    eprintln!("  gen --template api Agent");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "--help" | "-h" => { usage(); }

        "--template" | "-t" => {
            let name    = args.get(2).map(|s| s.as_str()).unwrap_or("crud");
            let subject = args.get(3).map(|s| s.as_str()).unwrap_or("Item");
            match templates::expand(name, subject) {
                Some(node) => {
                    let mut emit = Emitter::new();
                    println!("{}", emit.emit(&node));
                }
                None => {
                    eprintln!("Unknown template: {}  (try: crud api error repo entity)", name);
                    std::process::exit(1);
                }
            }
        }

        "--dag" => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            let node = parser::parse(&input);
            let mut emit = Emitter::new();
            println!("{}", emit.emit(&node));
        }

        "--dump" => {
            let expr = args.get(2).cloned().unwrap_or_default();
            let node = parser::parse(&expr);
            println!("{}", serde_json::to_string_pretty(&node).unwrap());
        }

        expr => {
            let node = parser::parse(expr);
            let mut emit = Emitter::new();
            println!("{}", emit.emit(&node));
        }
    }
}
