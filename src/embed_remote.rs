//! Remote text embedder targeting any OpenAI-compatible `/embeddings` HTTP API.
//!
//! Works with the official OpenAI API as well as drop-in replacements such as
//! Azure OpenAI, Ollama, vLLM, LocalAI, LM Studio, Together, Mistral, etc.
//! Endpoint: `POST {api_base}/embeddings` with the standard request body
//! `{ "model": "...", "input": "..." [, "dimensions": N] }`.
//!
//! # Examples
//!
//! ```
//! use open_ontologies::embed_remote::OpenAIEmbedder;
//! use std::time::Duration;
//!
//! // Construction is pure (no network call).
//! let embedder = OpenAIEmbedder::new(
//!     "https://api.openai.com/v1",
//!     Some("sk-test-key".to_string()),
//!     "text-embedding-3-small",
//!     Some(512),
//!     Duration::from_secs(30),
//! ).unwrap();
//!
//! assert_eq!(embedder.dim(), 512);
//! assert_eq!(embedder.model(), "text-embedding-3-small");
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::poincare::l2_normalize;

/// Default fallback dimension when the API response is empty before the first
/// successful call (e.g. text-embedding-3-small / ada-002 native dim).
const FALLBACK_DIM: usize = 1536;

#[derive(Debug, Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    /// Always request `float` encoding format (the default) — explicit for
    /// gateways that require it.
    encoding_format: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// OpenAI-compatible embeddings client.
///
/// # Examples
///
/// ```
/// use open_ontologies::embed_remote::OpenAIEmbedder;
/// use std::time::Duration;
///
/// // Build without a key — useful for local gateways (Ollama, vLLM, …).
/// let local = OpenAIEmbedder::new(
///     "http://localhost:11434/v1",
///     None,
///     "nomic-embed-text",
///     None,
///     Duration::from_secs(10),
/// ).unwrap();
///
/// // dim() returns FALLBACK_DIM (1536) until the first embed() call.
/// assert_eq!(local.dim(), 1536);
/// assert_eq!(local.model(), "nomic-embed-text");
/// ```
pub struct OpenAIEmbedder {
    client: reqwest::Client,
    /// Full URL including `/embeddings` path.
    endpoint: String,
    /// Bearer token sent in `Authorization` header. `None` skips the header
    /// — useful for local gateways without auth.
    api_key: Option<String>,
    model: String,
    /// Optional dimensions parameter to send. When `Some`, this is also the
    /// reported `dim()`. When `None`, dim is detected from the first
    /// response and cached.
    dimensions: Option<usize>,
    /// Cached output dimension (atomic so we can update from `&self`).
    detected_dim: AtomicUsize,
}

impl OpenAIEmbedder {
    /// Build an OpenAI-compatible embedder.
    ///
    /// `api_base` should not include the trailing `/embeddings` path —
    /// e.g. `https://api.openai.com/v1`.
    ///
    /// Empty or whitespace-only `api_key` values are treated as absent so
    /// no `Authorization` header is sent.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::embed_remote::OpenAIEmbedder;
    /// use std::time::Duration;
    ///
    /// // Trailing slash on api_base is normalised automatically.
    /// let e = OpenAIEmbedder::new(
    ///     "https://api.openai.com/v1/",
    ///     Some("sk-key".to_string()),
    ///     "text-embedding-ada-002",
    ///     None,
    ///     Duration::from_secs(60),
    /// ).unwrap();
    ///
    /// // Fallback dim before any embed() call.
    /// assert_eq!(e.dim(), 1536);
    ///
    /// // Blank key is treated as absent (no Authorization header).
    /// let no_auth = OpenAIEmbedder::new(
    ///     "http://localhost:8080/v1",
    ///     Some("   ".to_string()),   // whitespace-only → filtered out
    ///     "local-model",
    ///     Some(256),
    ///     Duration::from_secs(5),
    /// ).unwrap();
    /// assert_eq!(no_auth.dim(), 256);
    /// ```
    pub fn new(
        api_base: &str,
        api_key: Option<String>,
        model: impl Into<String>,
        dimensions: Option<usize>,
        request_timeout: Duration,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(request_timeout)
            .build()
            .context("failed to build reqwest client for OpenAI embeddings")?;

        let endpoint = format!("{}/embeddings", api_base.trim_end_matches('/'));

        // Reject obviously empty keys so they don't end up sending
        // `Authorization: Bearer `.
        let api_key = api_key.filter(|k| !k.trim().is_empty());

        let detected_dim = AtomicUsize::new(dimensions.unwrap_or(0));

        Ok(Self {
            client,
            endpoint,
            api_key,
            model: model.into(),
            dimensions,
            detected_dim,
        })
    }

    /// Embed a single text string. Returns an L2-normalized vector so it is
    /// directly comparable with embeddings produced by the local ONNX path.
    ///
    /// Returns an error with a structured description when:
    /// - the HTTP request fails (network error, DNS failure, timeout)
    /// - the API returns a non-2xx status (body truncated to 500 chars)
    /// - the response JSON cannot be parsed
    /// - the API returns an empty `data` array or a zero-length embedding
    ///
    /// ```no_run
    /// # use open_ontologies::embed_remote::OpenAIEmbedder;
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let embedder = OpenAIEmbedder::new(
    ///     "https://api.openai.com/v1",
    ///     Some(std::env::var("OPENAI_API_KEY").unwrap()),
    ///     "text-embedding-3-small",
    ///     Some(512),
    ///     Duration::from_secs(30),
    /// ).unwrap();
    ///
    /// let vec = embedder.embed("owl:Class ontology term").await.unwrap();
    /// // Output is L2-normalised: magnitude ≈ 1.0.
    /// let magnitude: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    /// assert!((magnitude - 1.0).abs() < 1e-4);
    /// assert_eq!(vec.len(), 512);
    /// # }
    /// ```
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let body = EmbeddingsRequest {
            model: &self.model,
            input: text,
            dimensions: self.dimensions,
            encoding_format: "float",
        };

        let mut req = self.client.post(&self.endpoint).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .with_context(|| format!("embedding request to {} failed", self.endpoint))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "embeddings API returned {}: {}",
                status,
                body.chars().take(500).collect::<String>()
            );
        }

        let parsed: EmbeddingsResponse = resp
            .json()
            .await
            .context("failed to parse embeddings response as JSON")?;

        let vec = parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("embeddings API returned empty data array"))?;

        if vec.is_empty() {
            anyhow::bail!("embeddings API returned a zero-length embedding");
        }

        // Cache the observed dimension on first success.
        if self.detected_dim.load(Ordering::Relaxed) == 0 {
            self.detected_dim.store(vec.len(), Ordering::Relaxed);
        }

        Ok(l2_normalize(&vec))
    }

    /// Output dimension.
    ///
    /// Returns the configured `dimensions` when set, then the dimension
    /// observed from the most recent successful `embed()` response, falling
    /// back to the standard 1536 of OpenAI's default embedding models if no
    /// call has been made yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::embed_remote::OpenAIEmbedder;
    /// use std::time::Duration;
    ///
    /// // Explicit dimensions → always returned as-is.
    /// let e = OpenAIEmbedder::new(
    ///     "https://api.openai.com/v1",
    ///     None,
    ///     "text-embedding-3-large",
    ///     Some(3072),
    ///     Duration::from_secs(30),
    /// ).unwrap();
    /// assert_eq!(e.dim(), 3072);
    ///
    /// // No dimensions + no prior call → fallback 1536.
    /// let e2 = OpenAIEmbedder::new(
    ///     "https://api.openai.com/v1",
    ///     None,
    ///     "text-embedding-ada-002",
    ///     None,
    ///     Duration::from_secs(30),
    /// ).unwrap();
    /// assert_eq!(e2.dim(), 1536);
    /// ```
    pub fn dim(&self) -> usize {
        if let Some(d) = self.dimensions {
            return d;
        }
        let observed = self.detected_dim.load(Ordering::Relaxed);
        if observed > 0 {
            observed
        } else {
            FALLBACK_DIM
        }
    }

    /// Configured model name (for diagnostics).
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::embed_remote::OpenAIEmbedder;
    /// use std::time::Duration;
    ///
    /// let e = OpenAIEmbedder::new(
    ///     "https://api.openai.com/v1",
    ///     None,
    ///     "text-embedding-3-small",
    ///     None,
    ///     Duration::from_secs(30),
    /// ).unwrap();
    /// assert_eq!(e.model(), "text-embedding-3-small");
    /// ```
    pub fn model(&self) -> &str {
        &self.model
    }
}
