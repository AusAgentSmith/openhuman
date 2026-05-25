//! Shared helpers for authenticated calls from the Tauri host to the active
//! core RPC endpoint.

use std::sync::LazyLock;

use parking_lot::RwLock;
use reqwest::RequestBuilder;
use url::Url;

const CORE_RPC_URL_ENV: &str = "OPENHUMAN_CORE_RPC_URL";
const CORE_RPC_TOKEN_ENV: &str = "OPENHUMAN_CORE_TOKEN";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CoreRpcConnectionKind {
    Local,
    Remote,
}

static CONNECTION_KIND: LazyLock<RwLock<CoreRpcConnectionKind>> =
    LazyLock::new(|| RwLock::new(CoreRpcConnectionKind::Local));

fn trimmed_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_local_rpc_url() -> String {
    format!(
        "http://127.0.0.1:{}/rpc",
        crate::core_process::default_core_port()
    )
}

pub(crate) fn core_rpc_url_value() -> String {
    trimmed_env(CORE_RPC_URL_ENV).unwrap_or_else(default_local_rpc_url)
}

pub(crate) fn initialize_connection_from_env_or_local(local_rpc_url: &str, local_port: u16) {
    if let Some(url) = trimmed_env(CORE_RPC_URL_ENV) {
        let url = normalize_core_rpc_url(&url).unwrap_or(url);
        if !is_local_core_url_for_port(&url, local_port) {
            std::env::set_var(CORE_RPC_URL_ENV, &url);
            *CONNECTION_KIND.write() = CoreRpcConnectionKind::Remote;
            log::info!(
                "[core] using externally configured core RPC endpoint {}",
                safe_url(&url)
            );
            return;
        }
    }

    configure_local_connection(local_rpc_url);
}

pub(crate) fn configure_local_connection(local_rpc_url: &str) {
    std::env::set_var(CORE_RPC_URL_ENV, local_rpc_url.trim());
    std::env::remove_var(CORE_RPC_TOKEN_ENV);
    *CONNECTION_KIND.write() = CoreRpcConnectionKind::Local;
    log::info!("[core] configured local embedded core RPC endpoint at {local_rpc_url}");
}

pub(crate) fn configure_remote_connection(url: &str, token: &str) -> Result<(), String> {
    let trimmed_url = url.trim();
    if trimmed_url.is_empty() {
        return Err("remote core RPC URL is required".to_string());
    }

    let normalized_url = normalize_core_rpc_url(trimmed_url)?;
    let parsed = Url::parse(&normalized_url)
        .map_err(|err| format!("remote core RPC URL is invalid: {err}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "remote core RPC URL must start with http:// or https://, got {scheme}:"
            ));
        }
    }

    let trimmed_token = token.trim();
    if trimmed_token.is_empty() {
        return Err("remote core RPC token is required".to_string());
    }

    std::env::set_var(CORE_RPC_URL_ENV, &normalized_url);
    std::env::set_var(CORE_RPC_TOKEN_ENV, trimmed_token);
    *CONNECTION_KIND.write() = CoreRpcConnectionKind::Remote;
    log::info!(
        "[core] configured remote core RPC endpoint at {}",
        safe_url(&normalized_url)
    );
    Ok(())
}

pub(crate) fn is_remote_connection() -> bool {
    *CONNECTION_KIND.read() == CoreRpcConnectionKind::Remote
}

pub(crate) fn remote_rpc_token_value() -> Option<String> {
    if is_remote_connection() {
        trimmed_env(CORE_RPC_TOKEN_ENV)
    } else {
        None
    }
}

fn auth_token_value() -> Option<String> {
    if is_remote_connection() {
        trimmed_env(CORE_RPC_TOKEN_ENV)
    } else {
        crate::core_process::current_rpc_token()
    }
}

pub(crate) fn apply_auth(builder: RequestBuilder) -> Result<RequestBuilder, String> {
    let token =
        auth_token_value().ok_or_else(|| "core RPC token is not initialized".to_string())?;
    Ok(builder.header("Authorization", format!("Bearer {token}")))
}

fn is_local_core_url_for_port(value: &str, port: u16) -> bool {
    let Ok(parsed) = Url::parse(value.trim()) else {
        return false;
    };
    if parsed.scheme() != "http" {
        return false;
    }
    let Some(host) = parsed
        .host_str()
        .map(|host| host.trim_matches(|c| c == '[' || c == ']'))
    else {
        return false;
    };
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return false;
    }
    if parsed.port_or_known_default() != Some(port) {
        return false;
    }
    parsed.path().trim_matches('/') == "rpc"
}

fn normalize_core_rpc_url(value: &str) -> Result<String, String> {
    let mut parsed =
        Url::parse(value.trim()).map_err(|err| format!("remote core RPC URL is invalid: {err}"))?;
    if parsed.path().is_empty() || parsed.path() == "/" {
        parsed.set_path("/rpc");
    }
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

fn safe_url(value: &str) -> String {
    match Url::parse(value) {
        Ok(mut url) => {
            url.set_username("").ok();
            url.set_password(None).ok();
            url.to_string()
        }
        Err(_) => "<invalid-url>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_core_rpc_url_appends_rpc_for_server_origin() {
        assert_eq!(
            normalize_core_rpc_url("https://openhuman.sprooty.com").unwrap(),
            "https://openhuman.sprooty.com/rpc"
        );
        assert_eq!(
            normalize_core_rpc_url("http://127.0.0.1:7788/").unwrap(),
            "http://127.0.0.1:7788/rpc"
        );
    }

    #[test]
    fn normalize_core_rpc_url_keeps_rpc_path_and_drops_query_fragment() {
        assert_eq!(
            normalize_core_rpc_url(" https://core.example.com/rpc?debug=1#top ").unwrap(),
            "https://core.example.com/rpc"
        );
    }

    #[test]
    fn local_core_url_detection_requires_same_loopback_port_and_rpc_path() {
        assert!(is_local_core_url_for_port(
            "http://127.0.0.1:7788/rpc",
            7788
        ));
        assert!(is_local_core_url_for_port(
            "http://localhost:7788/rpc/",
            7788
        ));
        assert!(!is_local_core_url_for_port(
            "https://127.0.0.1:7788/rpc",
            7788
        ));
        assert!(!is_local_core_url_for_port(
            "http://127.0.0.1:7789/rpc",
            7788
        ));
        assert!(!is_local_core_url_for_port(
            "https://openhuman.sprooty.com/rpc",
            7788
        ));
    }

    #[test]
    fn safe_url_strips_credentials_from_logs() {
        assert_eq!(
            safe_url("https://user:secret@openhuman.sprooty.com/rpc"),
            "https://openhuman.sprooty.com/rpc"
        );
    }
}
