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
