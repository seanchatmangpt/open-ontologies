//! R8-3 — Telemetry initialisation.
//!
//! Reads [`crate::config::TelemetryConfig`] and sets up the `tracing`
//! subscriber stack. When `otlp_endpoint` is configured, the intent is
//! to add an `opentelemetry-otlp` layer; for now the module wires the
//! existing `tracing-subscriber` env-filter + logging layer, with a
//! documented extension point for OTLP export in R9.
//!
//! All `tracing::debug!(target: "ontostar.*", ...)` spans emitted by the
//! admission gate (`src/admission.rs`) and the verifier worker
//! (`src/verifier_worker.rs`) are captured by this subscriber.
//!
//! # Configuration examples
//!
//! Construct a default config and verify field values:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let cfg = TelemetryConfig::default();
//! assert_eq!(cfg.service_name, "open-ontologies");
//! assert!(cfg.otlp_endpoint.is_none());
//! ```
//!
//! Construct a config with an OTLP endpoint:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let cfg = TelemetryConfig {
//!     otlp_endpoint: Some("http://otel-collector:4317".to_string()),
//!     service_name: "my-service".to_string(),
//! };
//! assert_eq!(cfg.service_name, "my-service");
//! assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://otel-collector:4317"));
//! ```
//!
//! Clone and compare configs:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let a = TelemetryConfig::default();
//! let b = a.clone();
//! assert_eq!(a.service_name, b.service_name);
//! assert_eq!(a.otlp_endpoint, b.otlp_endpoint);
//! ```
//!
//! Default OTLP endpoint is `None` — OTLP export is disabled out of the box:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let cfg = TelemetryConfig::default();
//! // Auto-instinct: OTLP is disabled by default so the server starts without
//! // requiring a collector to be running.
//! assert!(cfg.otlp_endpoint.is_none(), "OTLP must be off by default");
//! ```
//!
//! Service name from default config matches the binary name:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let cfg = TelemetryConfig::default();
//! assert_eq!(cfg.service_name, "open-ontologies");
//! // A custom service name can be set directly.
//! let custom = TelemetryConfig {
//!     service_name: "my-onto-service".to_string(),
//!     otlp_endpoint: None,
//! };
//! assert_eq!(custom.service_name, "my-onto-service");
//! ```
//!
//! `TelemetryConfig` is `Debug`-printable for structured logging:
//!
//! ```
//! use open_ontologies::config::TelemetryConfig;
//!
//! let cfg = TelemetryConfig::default();
//! let s = format!("{cfg:?}");
//! assert!(s.contains("service_name"), "Debug output must include field names");
//! assert!(s.contains("otlp_endpoint"));
//! ```
//!
//! Resolve telemetry service name (env-var-free path uses config value):
//!
//! ```
//! use open_ontologies::config::{TelemetryConfig, resolve_telemetry_service_name};
//!
//! // When env var is absent, the config value is used.
//! let cfg = TelemetryConfig {
//!     service_name: "my-resolver-test".to_string(),
//!     otlp_endpoint: None,
//! };
//! // Guard against the env var being set in the test environment.
//! if std::env::var("OPEN_ONTOLOGIES_SERVICE_NAME").is_err() {
//!     let name = resolve_telemetry_service_name(&cfg);
//!     assert_eq!(name, "my-resolver-test");
//! }
//! ```
//!
//! Resolve OTLP endpoint (env-var-free path returns `None` when config is `None`):
//!
//! ```
//! use open_ontologies::config::{TelemetryConfig, resolve_telemetry_otlp_endpoint};
//!
//! let cfg = TelemetryConfig::default();
//! if std::env::var("OPEN_ONTOLOGIES_OTLP_ENDPOINT").is_err() {
//!     let ep = resolve_telemetry_otlp_endpoint(&cfg);
//!     assert!(ep.is_none(), "no endpoint when config is None and env is absent");
//! }
//! ```

use crate::config::TelemetryConfig;

/// Initialise the global `tracing` subscriber from `cfg`.
///
/// Call once at startup (before the first span is emitted). Subsequent calls
/// are no-ops because `tracing_subscriber::set_global_default` returns an
/// error on the second call, which is silently ignored here.
///
/// # OTLP (R9 extension point)
///
/// When `cfg.otlp_endpoint` is `Some`, this function currently logs a
/// startup notice but does NOT yet wire the OTLP exporter. That wiring
/// requires `opentelemetry-otlp` + `tracing-opentelemetry` crate deps
/// (deferred to R9-3 once an endpoint is available for integration testing).
///
/// # Examples
///
/// Initialise with the default configuration (no OTLP endpoint):
///
/// ```
/// use open_ontologies::config::TelemetryConfig;
/// use open_ontologies::telemetry::init_telemetry;
///
/// let cfg = TelemetryConfig::default();
/// // Installs the tracing subscriber; second call is a no-op.
/// init_telemetry(&cfg);
/// init_telemetry(&cfg); // safe to call again
/// ```
///
/// Initialise with an OTLP endpoint (logs the deferred-wiring notice):
///
/// ```
/// use open_ontologies::config::TelemetryConfig;
/// use open_ontologies::telemetry::init_telemetry;
///
/// let cfg = TelemetryConfig {
///     otlp_endpoint: Some("http://localhost:4317".to_string()),
///     service_name: "my-service".to_string(),
/// };
/// init_telemetry(&cfg);
/// ```
///
/// `init_telemetry` is idempotent — repeated calls with different configs are
/// safe (subsequent subscribers are silently discarded):
///
/// ```
/// use open_ontologies::config::TelemetryConfig;
/// use open_ontologies::telemetry::init_telemetry;
///
/// let a = TelemetryConfig::default();
/// let b = TelemetryConfig {
///     service_name: "second-call".to_string(),
///     otlp_endpoint: None,
/// };
/// init_telemetry(&a);
/// init_telemetry(&b); // second call must not panic
/// ```
///
/// Calling with a config that has `otlp_endpoint = None` leaves tracing
/// active but without any OTLP export:
///
/// ```
/// use open_ontologies::config::TelemetryConfig;
/// use open_ontologies::telemetry::init_telemetry;
///
/// let cfg = TelemetryConfig { service_name: "no-otlp".to_string(), otlp_endpoint: None };
/// // Must not panic even when no external collector is present.
/// init_telemetry(&cfg);
/// assert!(cfg.otlp_endpoint.is_none());
/// ```
pub fn init_telemetry(cfg: &TelemetryConfig) {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    if let Some(endpoint) = cfg.otlp_endpoint.as_deref() {
        // R9-3 will replace this with real opentelemetry-otlp layer wiring.
        tracing::info!(
            otlp_endpoint = endpoint,
            service_name = cfg.service_name.as_str(),
            "OTLP endpoint configured; full exporter wiring deferred to R9-3",
        );
    }
}
