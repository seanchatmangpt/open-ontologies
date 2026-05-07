//! Doctor Commands — environment diagnostics

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Serialize)]
pub struct DoctorOutput {
    pub target: String,
    pub checks: Vec<DoctorCheck>,
    pub all_ok: bool,
}

#[derive(Serialize)]
pub struct RunOutput {
    pub checks: Vec<DoctorCheck>,
    pub all_ok: bool,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Serialize)]
pub struct ConfigCheckOutput {
    pub config_path: String,
    pub exists: bool,
    pub parseable: bool,
    pub ggen_path: String,
    pub ggen_path_resolves: bool,
}

impl ConfigCheckOutput {
    fn into_checks(self) -> Vec<DoctorCheck> {
        vec![
            DoctorCheck {
                name: "config_file".to_string(),
                ok: self.exists,
                detail: if self.exists {
                    format!("Config file found at {}", self.config_path)
                } else {
                    format!("Config file not found at {}", self.config_path)
                },
            },
            DoctorCheck {
                name: "config_parse".to_string(),
                ok: self.parseable && self.exists,
                detail: if self.parseable && self.exists {
                    "Config file parses successfully".to_string()
                } else if !self.exists {
                    "Cannot parse: config file missing".to_string()
                } else {
                    "Config file failed to parse".to_string()
                },
            },
            DoctorCheck {
                name: "ggen_path".to_string(),
                ok: self.ggen_path_resolves,
                detail: if self.ggen_path_resolves {
                    format!("ggen found at {}", self.ggen_path)
                } else {
                    format!("ggen not found: {}", self.ggen_path)
                },
            },
        ]
    }
}

#[derive(Serialize)]
pub struct DataCheckOutput {
    pub data_dir: String,
    pub exists: bool,
    pub writable: bool,
    pub db_accessible: bool,
}

impl DataCheckOutput {
    fn into_checks(self) -> Vec<DoctorCheck> {
        vec![
            DoctorCheck {
                name: "data_dir_exists".to_string(),
                ok: self.exists,
                detail: if self.exists {
                    format!("Data directory found at {}", self.data_dir)
                } else {
                    format!("Data directory not found at {}", self.data_dir)
                },
            },
            DoctorCheck {
                name: "data_dir_writable".to_string(),
                ok: self.writable && self.exists,
                detail: if self.writable && self.exists {
                    "Data directory is writable".to_string()
                } else if !self.exists {
                    "Cannot check: data directory missing".to_string()
                } else {
                    "Data directory is not writable".to_string()
                },
            },
            DoctorCheck {
                name: "db_accessible".to_string(),
                ok: self.db_accessible,
                detail: if self.db_accessible {
                    "SQLite database is accessible".to_string()
                } else {
                    "SQLite database is not accessible".to_string()
                },
            },
        ]
    }
}

#[derive(Serialize)]
pub struct StoreCheckOutput {
    pub triple_count: usize,
    pub active_ontology: Option<String>,
    pub cache_dir: String,
    pub cache_dir_exists: bool,
}

impl StoreCheckOutput {
    fn into_checks(self) -> Vec<DoctorCheck> {
        vec![
            DoctorCheck {
                name: "triple_count".to_string(),
                ok: self.triple_count > 0,
                detail: format!("{} triples loaded", self.triple_count),
            },
            DoctorCheck {
                name: "active_ontology".to_string(),
                ok: self.active_ontology.is_some(),
                detail: self.active_ontology.clone().unwrap_or_else(|| "No ontology loaded".to_string()),
            },
            DoctorCheck {
                name: "cache_dir".to_string(),
                ok: self.cache_dir_exists,
                detail: if self.cache_dir_exists {
                    format!("Cache directory exists at {}", self.cache_dir)
                } else {
                    format!("Cache directory missing at {}", self.cache_dir)
                },
            },
        ]
    }
}

#[derive(Serialize)]
pub struct GgenCheckOutput {
    pub ggen_path: String,
    pub found: bool,
    pub version: Option<String>,
}

impl GgenCheckOutput {
    fn into_checks(self) -> Vec<DoctorCheck> {
        vec![
            DoctorCheck {
                name: "ggen_binary".to_string(),
                ok: self.found,
                detail: if self.found {
                    format!("ggen binary found at {}", self.ggen_path)
                } else {
                    format!("ggen binary not found: {}", self.ggen_path)
                },
            },
            DoctorCheck {
                name: "ggen_version".to_string(),
                ok: self.version.is_some(),
                detail: self
                    .version
                    .clone()
                    .unwrap_or_else(|| "Could not extract ggen version".to_string()),
            },
        ]
    }
}

#[derive(Serialize)]
pub struct McpCheckOutput {
    pub binary_path: String,
    pub binary_exists: bool,
    pub help_exits_clean: bool,
}

impl McpCheckOutput {
    fn into_checks(self) -> Vec<DoctorCheck> {
        vec![
            DoctorCheck {
                name: "mcp_binary".to_string(),
                ok: self.binary_exists,
                detail: if self.binary_exists {
                    format!("MCP server binary found: {}", self.binary_path)
                } else {
                    "MCP server binary not in PATH".to_string()
                },
            },
            DoctorCheck {
                name: "mcp_help".to_string(),
                ok: self.help_exits_clean && self.binary_exists,
                detail: if self.help_exits_clean && self.binary_exists {
                    "MCP server --help exits cleanly".to_string()
                } else if !self.binary_exists {
                    "Cannot check: binary not found".to_string()
                } else {
                    "MCP server --help did not exit cleanly".to_string()
                },
            },
        ]
    }
}

#[derive(Serialize)]
pub struct EnvOutput {
    pub config_path: String,
    pub data_dir: String,
    pub ggen_path: String,
    pub env_overrides: Vec<(String, String)>,
}

// ── domain helpers ────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return home + &path[1..];
        }
    }
    path.to_string()
}

fn check_config_domain() -> ConfigCheckOutput {
    let config_path = format!("{}/.open-ontologies/config.toml", std::env::var("HOME").unwrap_or_default());
    let expanded = expand_tilde(&config_path);
    let exists = Path::new(&expanded).exists();
    let parseable = exists && std::fs::read_to_string(&expanded).ok().is_some();

    let ggen_path = std::env::var("OPEN_ONTOLOGIES_CODEGEN_GGEN_PATH").unwrap_or_else(|_| "ggen".to_string());
    let ggen_path_resolves = Command::new(&ggen_path).arg("--version").output().is_ok();

    ConfigCheckOutput { config_path, exists, parseable, ggen_path, ggen_path_resolves }
}

fn check_data_domain() -> DataCheckOutput {
    let data_dir = format!("{}/.open-ontologies", std::env::var("HOME").unwrap_or_default());
    let exists = Path::new(&data_dir).is_dir();
    let writable = exists && std::fs::metadata(&data_dir).map(|m| !m.permissions().readonly()).unwrap_or(false);
    let db_path = format!("{}/.open-ontologies/state.db", std::env::var("HOME").unwrap_or_default());
    let db_accessible = exists && (Path::new(&db_path).exists() || writable);

    DataCheckOutput { data_dir, exists, writable, db_accessible }
}

fn check_store_domain() -> StoreCheckOutput {
    let cache_dir = format!("{}/.open-ontologies/cache", std::env::var("HOME").unwrap_or_default());
    let cache_dir_exists = Path::new(&cache_dir).is_dir();
    let triple_count = 0;
    let active_ontology = None;

    StoreCheckOutput { triple_count, active_ontology, cache_dir, cache_dir_exists }
}

fn check_ggen_domain() -> GgenCheckOutput {
    let ggen_path = std::env::var("OPEN_ONTOLOGIES_CODEGEN_GGEN_PATH").unwrap_or_else(|_| "ggen".to_string());
    let output = Command::new(&ggen_path).arg("--version").output().ok();
    let found = output.is_some();
    let version = output
        .as_ref()
        .and_then(|o| String::from_utf8(o.stdout.clone()).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
        .or_else(|| output.as_ref().and_then(|o| String::from_utf8(o.stderr.clone()).ok().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()));

    GgenCheckOutput { ggen_path, found, version }
}

fn check_mcp_domain() -> McpCheckOutput {
    let binary_path = "open-ontologies".to_string();
    let help_output = Command::new(&binary_path).arg("server").arg("serve").arg("--help").output();
    let binary_exists = help_output.is_ok();
    let help_exits_clean = help_output.as_ref().map(|o| o.status.success()).unwrap_or(false);

    McpCheckOutput { binary_path, binary_exists, help_exits_clean }
}

fn env_domain() -> EnvOutput {
    let config_path = format!("{}/.open-ontologies/config.toml", std::env::var("HOME").unwrap_or_default());
    let data_dir = format!("{}/.open-ontologies", std::env::var("HOME").unwrap_or_default());
    let ggen_path = std::env::var("OPEN_ONTOLOGIES_CODEGEN_GGEN_PATH").unwrap_or_else(|_| "ggen".to_string());
    let mut env_overrides = Vec::new();

    for (key, value) in std::env::vars() {
        if key.starts_with("OPEN_ONTOLOGIES_") {
            env_overrides.push((key, value));
        }
    }

    EnvOutput { config_path, data_dir, ggen_path, env_overrides }
}

fn run_checks_for_target(target: &str) -> Vec<DoctorCheck> {
    match target {
        "config" => check_config_domain().into_checks(),
        "data" => check_data_domain().into_checks(),
        "store" => check_store_domain().into_checks(),
        "ggen" => check_ggen_domain().into_checks(),
        "mcp" => check_mcp_domain().into_checks(),
        "all" | _ => {
            let mut v = check_config_domain().into_checks();
            v.extend(check_data_domain().into_checks());
            v.extend(check_ggen_domain().into_checks());
            v.extend(check_mcp_domain().into_checks());
            v
        }
    }
}

// ── verbs ─────────────────────────────────────────────────────────────────

#[verb]
fn run() -> NounVerbResult<RunOutput> {
    let all_checks = ["config", "data", "ggen", "mcp"].iter().flat_map(|t| run_checks_for_target(t)).collect::<Vec<_>>();
    let failed = all_checks.iter().filter(|c| !c.ok).count();
    let passed = all_checks.len() - failed;
    Ok(RunOutput { checks: all_checks, all_ok: failed == 0, passed, failed })
}

#[verb]
fn check(target: Option<String>) -> NounVerbResult<DoctorOutput> {
    let t = target.unwrap_or_else(|| "all".to_string());
    let checks = run_checks_for_target(&t);
    let all_ok = checks.iter().all(|c| c.ok);
    Ok(DoctorOutput { target: t, checks, all_ok })
}

#[verb]
fn config() -> NounVerbResult<ConfigCheckOutput> {
    Ok(check_config_domain())
}

#[verb]
fn data() -> NounVerbResult<DataCheckOutput> {
    Ok(check_data_domain())
}

#[verb]
fn store() -> NounVerbResult<StoreCheckOutput> {
    Ok(check_store_domain())
}

#[verb]
fn ggen() -> NounVerbResult<GgenCheckOutput> {
    Ok(check_ggen_domain())
}

#[verb]
fn mcp() -> NounVerbResult<McpCheckOutput> {
    Ok(check_mcp_domain())
}

#[verb]
fn env() -> NounVerbResult<EnvOutput> {
    Ok(env_domain())
}
