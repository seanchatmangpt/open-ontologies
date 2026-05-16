//! AtomVM target generator — embedded Erlang for ESP32 / STM32 / RP2040.
//!
//! Emits `atomvm/<name>.erl` (the entry module with `start/0`) and
//! `atomvm/Makefile` (drives the `atomvm-pico-tool` / `mkimage` build
//! pipeline depending on `mcu_target`).

use super::{with_header, ManufacturedFile, SolutionSpec};

/// Generate AtomVM files (`.erl` module + `Makefile`) for supported MCU targets.
///
/// Returns an empty `Vec` when `mcu_target` is not one of the three supported
/// values; the admission gate treats that as [`DefectClass::GeneratorEmpty`].
///
/// # Examples
///
/// ```
/// use open_ontologies::manufacturing::{atomvm, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "blinky".into(),
///     description: "LED blinker".into(),
///     iac_target: "aws".into(),
///     region: "us-east-1".into(),
///     supervisor_children: 1,
///     mcu_target: "esp32".into(),
///     work_order_receipt_hash: "a".repeat(64),
/// };
/// let files = atomvm::generate(&spec);
/// assert_eq!(files.len(), 2);
/// assert!(files.iter().any(|f| f.path.ends_with(".erl")));
/// assert!(files.iter().any(|f| f.path.ends_with("Makefile")));
/// // Every file carries the receipt header.
/// assert!(files.iter().all(|f| f.contents.contains("ostar-artifact-hash:")));
/// ```
///
/// Unsupported MCU targets return an empty list:
///
/// ```
/// use open_ontologies::manufacturing::{atomvm, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "blinky".into(),
///     description: "LED blinker".into(),
///     iac_target: "aws".into(),
///     region: "us-east-1".into(),
///     supervisor_children: 1,
///     mcu_target: "msp430".into(),
///     work_order_receipt_hash: "b".repeat(64),
/// };
/// assert!(atomvm::generate(&spec).is_empty());
/// ```
///
/// Auto-instinct: all three supported MCU targets produce exactly 2 files each
/// and all files carry the receipt header:
///
/// ```
/// use open_ontologies::manufacturing::{atomvm, SolutionSpec};
///
/// for mcu in ["esp32", "stm32", "rp2040"] {
///     let spec = SolutionSpec {
///         name: "edge_node".into(),
///         description: "Edge node".into(),
///         iac_target: "aws".into(),
///         region: "us-east-1".into(),
///         supervisor_children: 1,
///         mcu_target: mcu.into(),
///         work_order_receipt_hash: "c".repeat(64),
///     };
///     let files = atomvm::generate(&spec);
///     assert_eq!(files.len(), 2, "mcu={mcu} should produce 2 files");
///     for f in &files {
///         assert!(
///             f.contents.contains("ostar-artifact-hash:"),
///             "{} on mcu={mcu} missing receipt header",
///             f.path
///         );
///     }
/// }
/// ```
///
/// Auto-instinct: the work-order receipt hash is embedded in the Erlang module
/// so an external auditor can read it from the AVM-loaded ROM:
///
/// ```
/// use open_ontologies::manufacturing::{atomvm, SolutionSpec};
///
/// let wor = "f".repeat(64);
/// let spec = SolutionSpec {
///     name: "sensor".into(),
///     description: "Sensor node".into(),
///     iac_target: "aws".into(),
///     region: "eu-west-1".into(),
///     supervisor_children: 1,
///     mcu_target: "rp2040".into(),
///     work_order_receipt_hash: wor.clone(),
/// };
/// let files = atomvm::generate(&spec);
/// let erl_file = files.iter().find(|f| f.path.ends_with(".erl")).unwrap();
/// // Receipt hash must appear in the Erlang source for audit round-trips.
/// assert!(erl_file.contents.contains(&wor));
/// // The target field is always "atomvm".
/// assert!(files.iter().all(|f| f.target == "atomvm"));
/// ```
pub fn generate(spec: &SolutionSpec) -> Vec<ManufacturedFile> {
    if !matches!(spec.mcu_target.as_str(), "esp32" | "stm32" | "rp2040") {
        return Vec::new();
    }
    vec![
        file(
            &format!("atomvm/{}.erl", spec.name),
            &generate_module_erl(spec),
            spec,
        ),
        file("atomvm/Makefile", &generate_makefile(spec), spec),
    ]
}

fn file(path: &str, body: &str, spec: &SolutionSpec) -> ManufacturedFile {
    ManufacturedFile {
        path: path.to_string(),
        contents: with_header(spec, path, body),
        target: "atomvm".to_string(),
    }
}

fn generate_module_erl(spec: &SolutionSpec) -> String {
    // AtomVM modules require a `start/0` entry point. We bind the
    // upstream work-order receipt as a module-level macro so an
    // external auditor can read it from the AVM-loaded ROM.
    format!(
        "-module({name}).\n\
         -export([start/0, work_order_receipt/0]).\n\
         \n\
         -define(WORK_ORDER_RECEIPT, \"{wor}\").\n\
         -define(MCU_TARGET, \"{mcu}\").\n\
         \n\
         start() ->\n\
         \x20\x20\x20\x20io:format(\"~s on ~s (work order: ~s)~n\",\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20[atom_to_list(?MODULE), ?MCU_TARGET, ?WORK_ORDER_RECEIPT]),\n\
         \x20\x20\x20\x20loop(0).\n\
         \n\
         loop(N) ->\n\
         \x20\x20\x20\x20receive\n\
         \x20\x20\x20\x20\x20\x20\x20\x20{{tick, From}} ->\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20From ! {{ack, N}},\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20loop(N + 1);\n\
         \x20\x20\x20\x20\x20\x20\x20\x20stop -> ok\n\
         \x20\x20\x20\x20after 1000 ->\n\
         \x20\x20\x20\x20\x20\x20\x20\x20loop(N + 1)\n\
         \x20\x20\x20\x20end.\n\
         \n\
         work_order_receipt() ->\n\
         \x20\x20\x20\x20?WORK_ORDER_RECEIPT.\n",
        name = spec.name,
        wor = spec.work_order_receipt_hash,
        mcu = spec.mcu_target,
    )
}

fn generate_makefile(spec: &SolutionSpec) -> String {
    let flash_target = match spec.mcu_target.as_str() {
        "esp32" => "esp32-flash",
        "stm32" => "stm32-flash",
        "rp2040" => "rp2040-flash",
        _ => unreachable!("validate_spec gates the variants"),
    };
    format!(
        ".PHONY: build flash clean\n\
         \n\
         MODULE = {name}\n\
         MCU = {mcu}\n\
         \n\
         build: $(MODULE).avm\n\
         \n\
         $(MODULE).avm: $(MODULE).beam\n\
         \tatomvm-mkimage --output $(MODULE).avm --module $(MODULE).beam\n\
         \n\
         $(MODULE).beam: $(MODULE).erl\n\
         \terlc $(MODULE).erl\n\
         \n\
         flash: {flash_target}\n\
         \n\
         {flash_target}: build\n\
         \tatomvm-flash --target $(MCU) --image $(MODULE).avm\n\
         \n\
         clean:\n\
         \trm -f $(MODULE).beam $(MODULE).avm\n",
        name = spec.name,
        mcu = spec.mcu_target,
        flash_target = flash_target,
    )
}
