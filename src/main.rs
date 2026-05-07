//! Open Ontologies CLI — entry point
//!
//! All noun-verb commands live in `cmds/` and are compiled as part of this binary.
//! clap-noun-verb discovers `#[verb]` functions via linkme distributed slices.

#![allow(non_upper_case_globals)] // linkme-generated statics
#![allow(clippy::unused_unit)]    // #[verb] macro generates unit expressions

mod cmds;

#[tokio::main]
async fn main() {
    match clap_noun_verb::run() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    }
}
