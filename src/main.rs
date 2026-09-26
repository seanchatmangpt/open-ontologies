//! Open Ontologies CLI — entry point
//!
//! All noun-verb commands live in `cmds/` and are compiled as part of this binary.
//! clap-noun-verb discovers `#[verb]` functions via linkme distributed slices.

#![allow(non_upper_case_globals)] // linkme-generated statics
#![allow(clippy::unused_unit)]    // #[verb] macro generates unit expressions

mod cmds;

/// Stack size for the thread that hosts the async runtime. Windows gives the
/// main thread 1 MiB of stack (vs 8 MiB on Linux/macOS), which overflows in
/// debug builds, so the CLI runs on a thread with an explicit 8 MiB.
const CLI_STACK_BYTES: usize = 8 * 1024 * 1024;

// Merge note (v26.9.26): upstream f146c229^2 introduced the stack-size thread
// around an `async_main`; the fork kept `#[tokio::main] async fn main`, and the
// merge left both (duplicate `main`, missing `async_main`). This keeps the
// fork's clap-noun-verb dispatch inside a Tokio runtime (same semantics as
// `#[tokio::main]`) and hosts it on the upstream 8 MiB thread.
fn main() {
    let code = std::thread::Builder::new()
        .name("open-ontologies-cli".into())
        .stack_size(CLI_STACK_BYTES)
        .spawn(run_cli)
        .expect("failed to spawn CLI thread")
        .join()
        .expect("CLI thread panicked");
    std::process::exit(code)
}

fn run_cli() -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("ERROR: failed to start async runtime: {}", e);
            return 1;
        }
    };
    runtime.block_on(async {
        match clap_noun_verb::run() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("ERROR: {}", e);
                1
            }
        }
    })
}
