//! LLM Boundary Translator (Groq-backed).
//!
//! **The translator proposes. It does not admit.**
//!
//! The translator converts messy stakeholder voice into candidate CTQ
//! structure (`CandidateCtq`). The deterministic CTQ admission gate then
//! admits or denies the candidate. LLM output is *provisional* until a
//! receipt is produced.
//!
//! # Invariant 7 — secret hygiene
//!
//! The resolved API key lives only on the [`GroqTranslator`] struct and
//! is bound to outbound requests via `bearer_auth`. It must never appear
//! in:
//!
//! - OCEL event attributes
//! - Receipt payloads
//! - Requirement / work-order records
//! - Counterfactual reports
//! - LLM prompt or response material persisted as evidence
//! - Error messages, debug logs, public projections, executive summaries
//!
//! The [`Debug`] impl redacts `api_key`. Outbound errors carry only the
//! HTTP status, the endpoint URL (no query string), and a length-capped
//! response body — never the request headers.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::LlmConfig;

/// Provisional candidate CTQ produced by the LLM boundary translator.
///
/// **Provisional** — this struct is not authoritative. It carries the
/// LLM's best attempt at translating messy voice into the 5 CTQ slots
/// the deterministic admission gate needs. Any of `measure_text`,
/// `verification_text`, `negative_case_text`, `control_plan_text` may be
/// empty; the gate will deny with `CtqIncomplete{missing}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateCtq {
    /// Echo of the source-voice signal (so reviewers can check the
    /// translator did not invent facts).
    pub source_voice_echo: String,
    /// Best-guess defect class tag. Free-form hint, not authoritative.
    pub defect_class_hint: String,
    /// CTQ statement (one sentence).
    pub ctq_text: String,
    /// Measurement description.
    pub measure_text: String,
    /// Verification method.
    pub verification_text: String,
    /// Negative case (what must be refused).
    pub negative_case_text: String,
    /// Control plan (regression prevention).
    pub control_plan_text: String,
    /// Always `true` — never set this to `false` in code; the gate is
    /// what makes a candidate authoritative, not the translator.
    pub provisional: bool,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    /// Force a JSON response when the model supports it. Ignored by
    /// gateways that don't honour the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// Groq-backed translator. Held by the MCP server; one per process is
/// fine because reqwest::Client is internally pooled.
pub struct GroqTranslator {
    client: reqwest::Client,
    /// Full URL including the `/chat/completions` path.
    endpoint: String,
    /// Bearer token. **Never logged.** When `None`, the translator refuses
    /// to call the remote and returns `Err(NoLlmConfigured)`.
    api_key: Option<String>,
    model: String,
}

impl std::fmt::Debug for GroqTranslator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redact api_key. The field never appears in any output of this impl.
        f.debug_struct("GroqTranslator")
            .field("endpoint", &self.endpoint)
            .field("api_key", &if self.api_key.is_some() { "<redacted>" } else { "<unset>" })
            .field("model", &self.model)
            .finish()
    }
}

impl GroqTranslator {
    /// Build a translator from an [`LlmConfig`]. The api_base resolves to
    /// `https://api.groq.com/openai/v1` by default; the api_key is
    /// resolved from `OPEN_ONTOLOGIES_LLM_API_KEY` then `GROQ_API_KEY`
    /// then the config field.
    pub fn from_config(cfg: &LlmConfig) -> Result<Self> {
        let api_base = crate::config::resolve_llm_api_base(cfg);
        let api_key = crate::config::resolve_llm_api_key(cfg);
        let model = crate::config::resolve_llm_model(cfg);
        let timeout = Duration::from_secs(cfg.request_timeout_secs.unwrap_or(30));
        Self::new(&api_base, api_key, model, timeout)
    }

    /// Build a translator from explicit parts. `api_base` should NOT
    /// include the trailing `/chat/completions` path.
    pub fn new(
        api_base: &str,
        api_key: Option<String>,
        model: impl Into<String>,
        request_timeout: Duration,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(request_timeout)
            .build()
            .context("failed to build reqwest client for Groq translator")?;
        let endpoint = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        // Reject obviously empty keys so they don't end up sending
        // `Authorization: Bearer `.
        let api_key = api_key.filter(|k| !k.trim().is_empty());
        Ok(Self { client, endpoint, api_key, model: model.into() })
    }

    /// True if an API key is configured. Used by the gate to deny with
    /// `LlmAuthorityClaimed` when the caller bypasses translation.
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Translate a source-voice signal into a provisional [`CandidateCtq`].
    ///
    /// Returns `Err(TranslateError::NoLlmConfigured)` if no API key is
    /// resolved — the caller (gate) must treat this as
    /// `LlmAuthorityClaimed` if a candidate proceeds without translation.
    pub async fn translate_candidate_ctq(&self, source_voice: &str) -> Result<CandidateCtq> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NoLlmConfigured: GROQ_API_KEY not set"))?;

        let system_prompt = "You are a Six-Sigma DFLSS analyst translating stakeholder \
            voice into candidate Critical-To-Quality (CTQ) structure. You are NOT \
            authoritative — your output is provisional and a deterministic gate will \
            admit or deny it. You MUST NOT invent facts beyond the input. Reply with a \
            JSON object containing exactly these keys: source_voice_echo, \
            defect_class_hint, ctq_text, measure_text, verification_text, \
            negative_case_text, control_plan_text. All values are strings.";

        let user_prompt = format!("Source voice: {source_voice}");

        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage { role: "system", content: system_prompt },
                ChatMessage { role: "user", content: &user_prompt },
            ],
            response_format: Some(ResponseFormat { kind: "json_object" }),
            temperature: 0.0,
        };

        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Groq request to {} failed", self.endpoint))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Cap the body so any echoed key (defensive — Groq does not
            // echo Authorization) is bounded; redact common Bearer
            // patterns.
            let redacted = redact_bearer_patterns(&body.chars().take(500).collect::<String>());
            anyhow::bail!("Groq API returned {}: {}", status, redacted);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .context("failed to parse Groq response as JSON")?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("Groq response had empty choices array"))?;

        let mut candidate: CandidateCtq = serde_json::from_str(&content).with_context(|| {
            format!(
                "Groq returned non-conforming JSON for CandidateCtq (length: {} chars)",
                content.len()
            )
        })?;
        // Force `provisional: true` regardless of what the LLM returned —
        // we do not let the LLM mark its own output authoritative.
        candidate.provisional = true;
        Ok(candidate)
    }

    /// Drive a DSPy-style **shaped** translation: compile the
    /// signature into a (system, user) prompt pair, send it to Groq,
    /// parse the response back through the shape's gauge, and refine
    /// up to `max_refinements` times on validation failure.
    ///
    /// Returns the admitted field map on success. Returns the FINAL
    /// validation failure list (not partial parses) on exhaust.
    ///
    /// The LLM is the *forming pressure*; the signature is the *mold*;
    /// the validator is the *gauge*. The downstream CTQ admission gate
    /// is what *admits* — this method only constrains.
    pub async fn translate_with_signature(
        &self,
        shape: &crate::signature_shape::SignatureShape,
        inputs: &std::collections::BTreeMap<String, String>,
        max_refinements: u32,
    ) -> Result<std::collections::BTreeMap<String, String>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NoLlmConfigured: GROQ_API_KEY not set"))?;

        let mut last_failures: Vec<crate::signature_shape::ValidationFailure> = Vec::new();

        for attempt in 0..=max_refinements {
            let (sys, user) = if attempt == 0 {
                shape.compile_prompt(inputs)
            } else {
                shape.compile_prompt_with_hints(inputs, &last_failures)
            };

            let body = ChatRequest {
                model: &self.model,
                messages: vec![
                    ChatMessage { role: "system", content: &sys },
                    ChatMessage { role: "user", content: &user },
                ],
                response_format: Some(ResponseFormat { kind: "json_object" }),
                temperature: 0.0,
            };

            let resp = self
                .client
                .post(&self.endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .with_context(|| format!("Groq request to {} failed", self.endpoint))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let redacted = redact_bearer_patterns(&body.chars().take(500).collect::<String>());
                anyhow::bail!("Groq API returned {}: {}", status, redacted);
            }

            let parsed: ChatResponse = resp
                .json()
                .await
                .context("failed to parse Groq response as JSON")?;
            let content = parsed
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| anyhow::anyhow!("Groq response had empty choices array"))?;

            match shape.parse_and_validate(&content) {
                Ok(fields) => return Ok(fields),
                Err(failures) => {
                    last_failures = failures;
                    // Continue to next refinement attempt.
                }
            }
        }
        // Out of refinements — surface the FINAL list of failures the
        // caller can serialize to OCEL / a denial response.
        let summary = last_failures
            .iter()
            .map(|f| f.revision_hint())
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!(
            "shaped translation failed after {} refinements: {summary}",
            max_refinements
        );
    }

    /// Endpoint URL (for diagnostics / OCEL attribute use). Never contains
    /// the API key.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Configured model (for diagnostics).
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Strip `Bearer <hex>` patterns from an arbitrary string. Defensive —
/// Groq does not echo the Authorization header in error responses, but
/// the redaction guards against gateway misbehaviour.
fn redact_bearer_patterns(s: &str) -> String {
    // Find "Bearer " followed by non-whitespace; replace the token with
    // "<redacted>". Simple linear scan; no regex dep.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    let needle = b"Bearer ";
    while i < bytes.len() {
        if bytes[i..].starts_with(needle) {
            out.push_str("Bearer <redacted>");
            i += needle.len();
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_redacts_api_key() {
        let t = GroqTranslator::new(
            "https://api.groq.com/openai/v1",
            Some("super-secret-key-DO-NOT-LEAK".to_string()),
            "llama-3.3-70b-versatile",
            Duration::from_secs(30),
        )
        .unwrap();
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("super-secret-key-DO-NOT-LEAK"));
        assert!(dbg.contains("<redacted>"));
    }

    #[test]
    fn debug_impl_marks_unset_when_no_key() {
        let t = GroqTranslator::new(
            "https://api.groq.com/openai/v1",
            None,
            "llama-3.3-70b-versatile",
            Duration::from_secs(30),
        )
        .unwrap();
        let dbg = format!("{t:?}");
        assert!(dbg.contains("<unset>"));
    }

    #[test]
    fn empty_or_whitespace_key_is_treated_as_unset() {
        let t = GroqTranslator::new(
            "https://api.groq.com/openai/v1",
            Some("   ".to_string()),
            "x",
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(!t.is_configured());
    }

    #[test]
    fn redact_bearer_patterns_replaces_token() {
        let s = "request failed with header Authorization: Bearer sk-abc123 more text";
        let r = redact_bearer_patterns(s);
        assert!(!r.contains("sk-abc123"));
        assert!(r.contains("Bearer <redacted>"));
    }

    #[test]
    fn redact_bearer_patterns_handles_no_bearer() {
        let s = "regular error text without auth headers";
        assert_eq!(redact_bearer_patterns(s), s);
    }

    #[test]
    fn endpoint_strips_trailing_slash() {
        let t = GroqTranslator::new(
            "https://api.groq.com/openai/v1/",
            None,
            "x",
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(t.endpoint(), "https://api.groq.com/openai/v1/chat/completions");
    }

    #[tokio::test]
    async fn translate_returns_no_llm_configured_when_key_missing() {
        let t = GroqTranslator::new(
            "https://api.groq.com/openai/v1",
            None,
            "x",
            Duration::from_secs(1),
        )
        .unwrap();
        let err = t.translate_candidate_ctq("voice").await.unwrap_err();
        assert!(err.to_string().contains("NoLlmConfigured"));
    }
}
