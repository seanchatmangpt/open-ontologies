use std::time::Duration;

/// Fire-and-forget webhook delivery. Timeout is configurable via
/// `[webhook] request_timeout_secs` (default 10s).
///
/// # Building webhook payloads
///
/// Payloads are plain `serde_json::Value` objects. Any JSON-serialisable
/// data can be used; round-tripping through JSON is lossless for scalar
/// types:
///
/// ```
/// use serde_json::json;
///
/// let payload = json!({
///     "event": "receipt_verified",
///     "seq": 42,
///     "artifact": "src/cmds/generated.rs"
/// });
///
/// // Field access
/// assert_eq!(payload["event"], "receipt_verified");
/// assert_eq!(payload["seq"], 42);
///
/// // JSON round-trip
/// let serialized = serde_json::to_string(&payload).unwrap();
/// let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
/// assert_eq!(parsed["seq"], 42);
/// ```
///
/// The optional `headers_json` parameter accepts a JSON object mapping header
/// names to values. `None` sends no extra headers:
///
/// ```
/// use serde_json::json;
///
/// // Valid headers_json string: a JSON object
/// let headers_json = r#"{"X-Custom-Header": "value", "Authorization": "Bearer tok"}"#;
/// let map: std::collections::HashMap<String, String> =
///     serde_json::from_str(headers_json).unwrap();
/// assert_eq!(map["X-Custom-Header"], "value");
/// assert_eq!(map["Authorization"], "Bearer tok");
/// ```
///
/// An empty payload is valid JSON and accepted by the function:
///
/// ```
/// let empty = serde_json::Value::Object(serde_json::Map::new());
/// assert!(empty.as_object().unwrap().is_empty());
/// ```
///
/// # Example (live network)
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() {
/// use serde_json::json;
///
/// // POST a JSON payload to a webhook endpoint; returns Ok(()) on 2xx.
/// open_ontologies::webhook::deliver_webhook(
///     "https://example.com/hook",
///     Some(r#"{"X-Custom-Header":"value"}"#),
///     &json!({"event": "receipt_verified", "seq": 42}),
/// )
/// .await
/// .unwrap();
/// # }
/// ```
pub async fn deliver_webhook(
    url: &str,
    headers_json: Option<&str>,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timeout_secs = crate::runtime::webhook_request_timeout_secs();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    let mut req = client.post(url).json(payload);
    if let Some(hdr_json) = headers_json
        && let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(hdr_json) {
            for (k, v) in map {
                req = req.header(&k, &v);
            }
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        eprintln!("Webhook to {} returned {}", url, status);
    }
    Ok(())
}
