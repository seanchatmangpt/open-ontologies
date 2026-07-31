//! Open Ontologies CLI — entry point
//!
//! All noun-verb commands live in `cmds/` and are compiled as part of this binary.
//! clap-noun-verb discovers `#[verb]` functions via linkme distributed slices.

#![allow(non_upper_case_globals)] // linkme-generated statics
#![allow(clippy::unused_unit)] // #[verb] macro generates unit expressions

mod cmds;

fn main() -> anyhow::Result<()> {
    // The root async future is polled on the spawned thread. Windows gives the
    // process main thread a smaller stack than Linux/macOS, so preserve the
    // explicit 8 MiB boundary used by upstream.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(async_main)?
        .join()
        .expect("main thread panicked")
}

#[tokio::main]
async fn async_main() -> anyhow::Result<()> {
    clap_noun_verb::run().map_err(|error| anyhow::anyhow!(error.to_string()))
}
