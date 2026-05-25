//! Migration 1 → 2: unify the scattered AI provider settings into the new
//! per-workload provider-string fields, and seed the `cloud_providers` list.
//!
//! ## What this migration consolidates
//!
//! Pre-unification config carried five different vocabularies for the same
//! question — *"where does this LLM workload run?"*:
//!
//! - `inference_url` + `model_routes`        — global external-provider preset
//! - `reasoning_provider` / `agentic_provider` / `coding_provider`
//!                                          — per-role chat (#1710, partial)
//! - `local_ai.usage.{embeddings,heartbeat,learning_reflection,subconscious}`
//!                                          — legacy local-vs-remote booleans
//! - `memory_tree.llm_backend` (+ `cloud_llm_model`) — memory summariser
//!
//! After this migration there is one grammar — provider strings parsed by
//! [`crate::openhuman::inference::provider::factory`] — addressing all eight workloads
//! uniformly:
//!
//! ```text
//! reasoning_provider, agentic_provider, coding_provider,
//! memory_provider,    embeddings_provider, heartbeat_provider,
//! learning_provider,  subconscious_provider
//! ```
//!
//! plus `cloud_providers: Vec<CloudProviderCreds>` and `primary_cloud` for
//! the credential side.
//!
//! ## Behaviour
//!
//! - Pure in-memory mutation of `Config`. The caller (`migrations::run_pending`)
//!   persists the result via `Config::save()` and bumps `schema_version`.
//! - Idempotent: gated on `*.is_none()` per field. A re-run after a previous
//!   successful run is a no-op.
//! - Never touches keys / secrets. API keys remain in
//!   `auth-profiles.json` via [`crate::openhuman::credentials::AuthService`].
//! - Always seeds an OpenAI entry into `cloud_providers` (idempotent —
//!   only when the list is empty).
//! - Migrates `inference_url` into a `Custom` cloud provider entry when the
//!   URL doesn't look like the OpenHuman backend.

use crate::openhuman::config::schema::cloud_providers::{
    generate_provider_id, AuthStyle, CloudProviderCreds,
};
use crate::openhuman::config::Config;

/// Counters returned by [`run`] for diagnostics. Logged at INFO once per
/// successful migration run.
#[derive(Debug, Default, Clone)]
pub struct MigrationStats {
    pub cloud_providers_seeded: usize,
    pub primary_cloud_set: bool,
    pub workload_fields_filled: usize,
}

/// Run the AI-provider unification migration on the given `Config`.
///
/// Synchronous because the body is pure config mutation — no I/O. The caller
/// is responsible for persisting via `Config::save()` once the runner has
/// also bumped `schema_version`.
pub fn run(config: &mut Config) -> anyhow::Result<MigrationStats> {
    let mut stats = MigrationStats::default();

    seed_cloud_providers(config, &mut stats);
    set_primary_cloud(config, &mut stats);
    derive_workload_providers(config, &mut stats);

    log::info!(
        "[migrations][unify-ai] done seeded_providers={} primary_set={} workload_fields_filled={}",
        stats.cloud_providers_seeded,
        stats.primary_cloud_set,
        stats.workload_fields_filled,
    );
    Ok(stats)
}

/// Seed `cloud_providers` with the built-in OpenAI entry the user can
/// route against by storing their key under the `openai` slug, plus a
/// Custom entry derived from any legacy `inference_url`.
fn seed_cloud_providers(config: &mut Config, stats: &mut MigrationStats) {
    if !config.cloud_providers.is_empty() {
        log::debug!(
            "[migrations][unify-ai] cloud_providers already populated ({} entries), skipping seed",
            config.cloud_providers.len()
        );
        return;
    }

    // After the local-OAuth refactor the OpenHuman backend is gone; the
    // single default entry is OpenAI, matching `DEFAULT_MODEL =
    // "openai:gpt-5.4"`. User stores their key against this slug via
    // `auth_store_provider_credentials`.
    config.cloud_providers.push(CloudProviderCreds {
        id: generate_provider_id("openai"),
        slug: "openai".to_string(),
        label: "OpenAI".to_string(),
        endpoint: "https://api.openai.com/v1".to_string(),
        auth_style: AuthStyle::Bearer,
        default_model: Some("gpt-5.4".to_string()),
        ..Default::default()
    });
    stats.cloud_providers_seeded += 1;

    // If there's a legacy `inference_url` pointing at a non-OpenHuman
    // endpoint, surface it as a Custom entry so the user keeps their
    // configuration. The actual key continues to live in auth-profiles.json
    // (or in `api_key` on Config — which is OpenHuman's session JWT and
    // doesn't apply here; users will re-enter via the new UI).
    if let Some(raw) = config.inference_url.as_deref() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && !looks_like_openhuman(trimmed) {
            // Derive a sensible default model from the legacy model_routes
            // (prefer "reasoning" hint, fall back to whatever is set).
            let default_model = config
                .model_routes
                .iter()
                .find(|r| r.hint.eq_ignore_ascii_case("reasoning"))
                .or_else(|| config.model_routes.first())
                .map(|r| r.model.clone())
                .filter(|m| !m.is_empty());
            config.cloud_providers.push(CloudProviderCreds {
                id: generate_provider_id("custom"),
                slug: "custom".to_string(),
                label: "Custom".to_string(),
                endpoint: trimmed.to_string(),
                auth_style: AuthStyle::Bearer,
                default_model,
                ..Default::default()
            });
            stats.cloud_providers_seeded += 1;
            log::info!(
                "[migrations][unify-ai] seeded Custom cloud_providers entry from legacy \
                 inference_url_present=true"
            );
        }
    }
}

/// Default `primary_cloud` to the OpenAI entry (the first one we just
/// seeded, by construction). Idempotent — only sets if currently `None`.
fn set_primary_cloud(config: &mut Config, stats: &mut MigrationStats) {
    if config.primary_cloud.is_some() {
        return;
    }
    let entry = config.cloud_providers.iter().find(|e| e.slug == "openai");
    if let Some(entry) = entry {
        config.primary_cloud = Some(entry.id.clone());
        stats.primary_cloud_set = true;
        log::debug!(
            "[migrations][unify-ai] primary_cloud set to openai entry id={}",
            entry.id
        );
    }
}

/// Derive each per-workload `*_provider` field from the legacy flags.
///
/// All fields are gated on `is_none()` — a partially-migrated config skips
/// fields that were already set by a previous run or a hand-edit.
fn derive_workload_providers(config: &mut Config, stats: &mut MigrationStats) {
    // The default config keeps `runtime_enabled` true so the local-runtime
    // settings panel can inspect Ollama when present, but workload routing
    // must not silently depend on Ollama unless the user explicitly opted in.
    let runtime_on = config.local_ai.runtime_enabled && config.local_ai.opt_in_confirmed;
    let chat_model = config.local_ai.chat_model_id.clone();
    let embed_model = config.local_ai.embedding_model_id.clone();

    let set_field = |field: &mut Option<String>, value: String, stats: &mut MigrationStats| {
        if field.is_none() {
            *field = Some(value);
            stats.workload_fields_filled += 1;
        }
    };

    // Memory summariser — preserve explicit local routing. Remote/external
    // routing is represented by leaving the field unset; the factory then
    // resolves the user's configured external provider. Do not write the old
    // "cloud" sentinel because the hosted OpenHuman backend is gone.
    if matches!(
        config.memory_tree.llm_backend,
        crate::openhuman::config::schema::LlmBackend::Local
    ) && runtime_on
        && !chat_model.is_empty()
    {
        set_field(
            &mut config.memory_provider,
            format!("ollama:{}", chat_model),
            stats,
        );
    }

    // Embeddings — uses the embedding_model_id, not chat_model_id.
    if config.local_ai.usage.embeddings && runtime_on && !embed_model.is_empty() {
        set_field(
            &mut config.embeddings_provider,
            format!("ollama:{}", embed_model),
            stats,
        );
    }

    // The remaining three use the chat model when local; otherwise they remain
    // unset and resolve to the configured external provider at runtime.
    if config.local_ai.usage.heartbeat && runtime_on && !chat_model.is_empty() {
        set_field(
            &mut config.heartbeat_provider,
            format!("ollama:{}", chat_model),
            stats,
        );
    }

    if config.local_ai.usage.learning_reflection && runtime_on && !chat_model.is_empty() {
        set_field(
            &mut config.learning_provider,
            format!("ollama:{}", chat_model),
            stats,
        );
    }

    if config.local_ai.usage.subconscious && runtime_on && !chat_model.is_empty() {
        set_field(
            &mut config.subconscious_provider,
            format!("ollama:{}", chat_model),
            stats,
        );
    }

    // The chat workloads intentionally stay None; Settings → AI stores their
    // shared default in `default_model`, and the factory resolves unset chat
    // roles through that external-provider route.
}

/// Heuristic: does the URL look like the removed hosted provider endpoint?
///
/// Used to decide whether a non-empty legacy `inference_url` should be
/// migrated into a Custom cloud provider entry. Removed hosted endpoints are
/// ignored so the upgrade does not preserve a dead backend route.
///
/// Matches only on the host component to avoid false positives from custom
/// endpoints that happen to contain "openhuman" in a path or query string.
fn looks_like_openhuman(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    // Strip scheme if present.
    let without_scheme = lower.split("://").nth(1).unwrap_or(&lower);
    // Strip userinfo and take only the host[:port] part before any path.
    let authority = without_scheme.split('/').next().unwrap_or("");
    let host = authority.split('@').last().unwrap_or(authority);
    let host_no_port = host.split(':').next().unwrap_or(host);
    host_no_port == format!("api.{}", "openhuman.ai")
        || host_no_port.ends_with(".openhuman.ai")
        // Allow bare "openhuman" for local/dev names (e.g. Docker compose service names).
        || host_no_port == "openhuman"
}

#[cfg(test)]
#[path = "unify_ai_provider_settings_tests.rs"]
mod tests;
