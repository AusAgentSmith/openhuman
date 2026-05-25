//! Build an [`Embedder`] from [`Config`] settings.
//!
//! Resolution order:
//! 1. **Explicit override** — `memory_tree.embedding_endpoint` +
//!    `memory_tree.embedding_model` both Some → [`OllamaEmbedder`] with
//!    those exact values. For power users / E2E test rigs that want to
//!    point at a non-default Ollama endpoint.
//! 2. **Local-AI usage flag** — `config.local_ai.use_local_for_embeddings()`
//!    (i.e. `runtime_enabled && usage.embeddings`) → [`OllamaEmbedder`]
//!    against [`ollama_base_url`] with the user's chosen
//!    `config.local_ai.embedding_model_id`. This is the path driven by
//!    the "Memory embeddings" checkbox in Local AI Settings.
//! 3. **Non-strict fallback** — when no embedding provider is configured and
//!    `memory_tree.embedding_strict = false`, use [`InertEmbedder`] so the
//!    memory tree can still build without semantic reranking.
//! 4. **Strict default** — hard error. Embeddings must be configured explicitly
//!    through local Ollama or an endpoint/model override.
//!
//! NOTE on dimensions: the memory tree on-disk format is hard-coded at
//! [`EMBEDDING_DIM`](super::EMBEDDING_DIM) (1024). If the user picks a
//! local embedding model whose output is a different dimensionality,
//! the trait's post-call validator rejects each embed with a clear
//! `expected N dims, got M` error. Switching the local model picker in
//! Local AI Settings is the fix.
//!
//! Env var overrides applied in [`crate::openhuman::config::load`]:
//! - `OPENHUMAN_MEMORY_EMBED_ENDPOINT`
//! - `OPENHUMAN_MEMORY_EMBED_MODEL`
//! - `OPENHUMAN_MEMORY_EMBED_TIMEOUT_MS`

use anyhow::Result;

use super::{Embedder, InertEmbedder, OllamaEmbedder};
use crate::openhuman::config::Config;
use crate::openhuman::inference::local::ollama_base_url;

/// Construct the active embedder for this process, honouring
/// `config.memory_tree.*` and `embedding_strict`.
///
/// Returns a boxed trait object so ingest / seal can call one code path
/// regardless of which provider is active. The returned box is created
/// per call — cheap because `OllamaEmbedder` owns a cloned `reqwest::Client`
/// internally and `InertEmbedder` is a ZST.
pub fn build_embedder_from_config(config: &Config) -> Result<Box<dyn Embedder>> {
    let tree_cfg = &config.memory_tree;
    match (
        tree_cfg.embedding_endpoint.as_deref(),
        tree_cfg.embedding_model.as_deref(),
    ) {
        (Some(endpoint), Some(model))
            if !endpoint.trim().is_empty() && !model.trim().is_empty() =>
        {
            let timeout_ms = tree_cfg.embedding_timeout_ms.unwrap_or(0);
            log::debug!(
                "[memory_tree::embed::factory] using Ollama endpoint={} model={} timeout_ms={}",
                endpoint,
                model,
                timeout_ms
            );
            Ok(Box::new(OllamaEmbedder::new(
                endpoint.to_string(),
                model.to_string(),
                timeout_ms,
            )))
        }
        _ => {
            // Honour the unified AI settings: `embeddings_provider` is the
            // single source of truth. When it parses as `ollama:<model>` we
            // route locally; otherwise we fall back to the cloud session.
            if let Some(model) = config.workload_local_model("embeddings") {
                let endpoint = ollama_base_url();
                let timeout_ms = tree_cfg.embedding_timeout_ms.unwrap_or(0);
                log::debug!(
                    "[memory_tree::embed::factory] embeddings_provider=ollama:{} — using local Ollama endpoint={} timeout_ms={}",
                    model, endpoint, timeout_ms
                );
                Ok(Box::new(OllamaEmbedder::new(endpoint, model, timeout_ms)))
            } else if !tree_cfg.embedding_strict {
                log::warn!(
                    "[memory_tree::embed::factory] no embedding provider configured; \
                     using inert zero-vector embedder because memory_tree.embedding_strict=false"
                );
                Ok(Box::new(InertEmbedder::new()))
            } else {
                anyhow::bail!(
                    "No embedding provider configured. Enable local AI embeddings or set \
                     memory_tree.embedding_endpoint and memory_tree.embedding_model in config.toml."
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().to_path_buf();
        // Plant config_path in the tempdir so cloud_session_available()
        // checks a writable directory; tests that need to simulate a
        // logged-in user just `touch` auth-profiles.json next to it.
        cfg.config_path = tmp.path().join("config.toml");
        (tmp, cfg)
    }

    fn expect_embedder_err(
        result: anyhow::Result<Box<dyn Embedder>>,
        context: &str,
    ) -> anyhow::Error {
        match result {
            Ok(embedder) => panic!("{context}: expected error, got {}", embedder.name()),
            Err(err) => err,
        }
    }

    #[test]
    fn ollama_chosen_when_endpoint_and_model_set() {
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = Some("http://localhost:11434".into());
        cfg.memory_tree.embedding_model = Some("bge-m3".into());
        cfg.memory_tree.embedding_timeout_ms = Some(5000);
        let e = build_embedder_from_config(&cfg).expect("Ollama path should build");
        assert_eq!(e.name(), "ollama");
    }

    #[test]
    fn unset_endpoint_without_provider_uses_inert_when_not_strict() {
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = None;
        cfg.memory_tree.embedding_model = None;
        cfg.memory_tree.embedding_strict = false;
        let e = build_embedder_from_config(&cfg).expect("inert fallback should build");
        assert_eq!(e.name(), "inert");
    }

    #[test]
    fn empty_strings_count_as_unset_and_use_inert_when_not_strict() {
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = Some("".into());
        cfg.memory_tree.embedding_model = Some("".into());
        cfg.memory_tree.embedding_strict = false;
        let e = build_embedder_from_config(&cfg).expect("inert fallback should build");
        assert_eq!(e.name(), "inert");
    }

    #[test]
    fn strict_mode_unset_provider_errors() {
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = None;
        cfg.memory_tree.embedding_model = None;
        cfg.memory_tree.embedding_strict = true;
        let err = expect_embedder_err(build_embedder_from_config(&cfg), "unset provider");
        assert!(err.to_string().contains("No embedding provider configured"));
    }

    #[test]
    fn local_ai_usage_embeddings_routes_to_ollama() {
        // After #1710 the local-vs-cloud decision for embeddings is
        // driven by `embeddings_provider` (via
        // `Config::workload_uses_local("embeddings")`), not the legacy
        // `local_ai.usage.embeddings` flag. Set the new workload field
        // so the local branch is taken; `embedding_model_id` is still
        // the model name source for the Ollama provider.
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = None;
        cfg.memory_tree.embedding_model = None;
        cfg.embeddings_provider = Some("ollama:all-minilm:latest".into());
        cfg.local_ai.runtime_enabled = true;
        cfg.local_ai.embedding_model_id = "all-minilm:latest".to_string();
        let e = build_embedder_from_config(&cfg).expect("ollama path should build");
        assert_eq!(e.name(), "ollama");
    }

    #[test]
    fn local_ai_usage_off_without_provider_uses_inert_when_not_strict() {
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = None;
        cfg.memory_tree.embedding_model = None;
        cfg.local_ai.runtime_enabled = true;
        cfg.local_ai.usage.embeddings = false;
        cfg.memory_tree.embedding_strict = false;
        let e = build_embedder_from_config(&cfg).expect("inert fallback should build");
        assert_eq!(e.name(), "inert");
    }

    #[test]
    fn explicit_endpoint_override_wins_over_local_ai_flag() {
        // Power-user override beats the checkbox.
        let (_tmp, mut cfg) = test_config();
        cfg.memory_tree.embedding_endpoint = Some("http://staging-embed:11434".into());
        cfg.memory_tree.embedding_model = Some("bge-m3".into());
        cfg.local_ai.runtime_enabled = true;
        cfg.local_ai.usage.embeddings = true;
        let e = build_embedder_from_config(&cfg).expect("override path should build");
        assert_eq!(e.name(), "ollama");
    }
}
