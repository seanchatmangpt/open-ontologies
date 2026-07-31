use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn real_wasm4pm_crates_are_the_only_runtime_authority() {
    let root = root();
    let manifest = fs::read_to_string(root.join("ggen.toml")).expect("read ggen.toml");
    for forbidden in [
        "wasm4pm-algos-stub",
        "wasm4pm-cognition-stub",
        "wasm4pm-stub",
        "wasm4pm-types-stub",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "duplicate ggen authority returned: {forbidden}"
        );
    }
    for path in [
        "src/wasm4pm_algos_stub.rs",
        "src/wasm4pm_cognition_stub.rs",
        "src/wasm4pm_stub.rs",
        "src/wasm4pm_types_stub.rs",
    ] {
        assert!(
            !root.join(path).exists(),
            "duplicate generated authority exists: {path}"
        );
    }
    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo.contains("package = \"wasm4pm-compat\""));
    assert!(cargo.contains("wasm4pm = { git = \"https://github.com/seanchatmangpt/wasm4pm\""));
    assert!(
        cargo.contains("wasm4pm-cognition = { git = \"https://github.com/seanchatmangpt/wasm4pm\"")
    );
}
