//! cargo-cicd binary target

#![allow(non_upper_case_globals)] // linkme-generated statics
#![allow(clippy::unused_unit)] // #[verb] macro generates unit expressions

#[path = "../cmds/mod.rs"]
mod cmds;

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "cicd" {
            // Already has "cicd", do nothing
        } else {
            args.insert(1, "cicd".to_string());
        }
    } else {
        args.push("cicd".to_string());
    }

    let registry = clap_noun_verb::cli::registry::CommandRegistry::get();
    let registry = registry.lock().unwrap_or_else(|e| e.into_inner());
    match registry.run(args) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    }
}
