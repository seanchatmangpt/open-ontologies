//! Open Ontologies CLI — entry point
//!
//! All noun-verb commands live in `cmds/` and are compiled as part of this binary.
//! clap-noun-verb discovers `#[verb]` functions via linkme distributed slices.

#![allow(non_upper_case_globals)] // linkme-generated statics
#![allow(clippy::unused_unit)]    // #[verb] macro generates unit expressions

mod cmds;

fn main() -> anyhow::Result<()> {
    // The root async future is polled on the calling thread. Windows gives
    // the main thread 1 MiB of stack (vs 8 MiB on Linux/macOS), which
    // overflows in debug builds, so run on a thread with an explicit 8 MiB.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(async_main)?
        .join()
        .expect("main thread panicked")
}

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
