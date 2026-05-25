//! JSON-RPC / CLI controller surface for security policy introspection.

use serde_json::json;

use crate::openhuman::security::SecurityPolicy;
use crate::rpc::RpcOutcome;

pub fn security_policy_info() -> RpcOutcome<serde_json::Value> {
    let policy = SecurityPolicy::default();
    let payload = json!({
        "autonomy": policy.autonomy,
        "workspace_only": policy.workspace_only,
        "allowed_commands": policy.allowed_commands,
        "max_actions_per_hour": policy.max_actions_per_hour,
        "require_approval_for_medium_risk": policy.require_approval_for_medium_risk,
        "block_high_risk_commands": policy.block_high_risk_commands,
    });
    RpcOutcome::single_log(payload, "security_policy_info computed")
}

pub fn client_sessions_status() -> RpcOutcome<serde_json::Value> {
    let summary = crate::openhuman::security::client_sessions::global_summary();
    let payload = json!({
        "session_model": "static_bearer_plus_device_sessions",
        "device_scoped_tokens": true,
        "revocation_supported": true,
        "static_bearer_enabled": true,
        "provider_tokens_server_side": true,
        "mobile_public_ready": false,
        "recommended_next_step": "Keep the bootstrap bearer for local admin only; use named client sessions for desktop/mobile and add TLS enforcement before public mobile use.",
        "client_token_storage": "hashed_device_tokens",
        "provider_token_storage": "server_auth_service",
        "sessions": summary,
    });
    RpcOutcome::single_log(payload, "security_client_sessions_status computed")
}

pub fn client_sessions_create(label: Option<&str>) -> RpcOutcome<serde_json::Value> {
    match crate::openhuman::security::client_sessions::issue_global(label) {
        Ok(created) => RpcOutcome::single_log(
            serde_json::to_value(created)
                .unwrap_or_else(|_| json!({ "error": "serialize client session" })),
            "security_client_sessions_create issued",
        ),
        Err(err) => RpcOutcome::single_log(
            json!({ "error": err.to_string() }),
            "security_client_sessions_create failed",
        ),
    }
}

pub fn client_sessions_list() -> RpcOutcome<serde_json::Value> {
    match crate::openhuman::security::client_sessions::list_global() {
        Ok(sessions) => RpcOutcome::single_log(
            json!({ "sessions": sessions }),
            "security_client_sessions_list returned",
        ),
        Err(err) => RpcOutcome::single_log(
            json!({ "error": err.to_string(), "sessions": [] }),
            "security_client_sessions_list failed",
        ),
    }
}

pub fn client_sessions_revoke(session_id: &str) -> RpcOutcome<serde_json::Value> {
    if session_id.trim().is_empty() {
        return RpcOutcome::single_log(
            json!({ "revoked": false, "error": "session_id must not be empty" }),
            "security_client_sessions_revoke rejected",
        );
    }

    match crate::openhuman::security::client_sessions::revoke_global(session_id) {
        Ok(Some(session)) => RpcOutcome::single_log(
            json!({ "revoked": true, "session": session }),
            "security_client_sessions_revoke revoked",
        ),
        Ok(None) => RpcOutcome::single_log(
            json!({ "revoked": false, "error": "session not found" }),
            "security_client_sessions_revoke not_found",
        ),
        Err(err) => RpcOutcome::single_log(
            json!({ "revoked": false, "error": err.to_string() }),
            "security_client_sessions_revoke failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_policy_info_returns_all_documented_fields() {
        // Locks in the JSON shape the JSON-RPC clients depend on —
        // any rename / removal of a field would break the UI.
        let outcome = security_policy_info();
        for key in [
            "autonomy",
            "workspace_only",
            "allowed_commands",
            "max_actions_per_hour",
            "require_approval_for_medium_risk",
            "block_high_risk_commands",
        ] {
            assert!(
                outcome.value.get(key).is_some(),
                "missing `{key}` in security_policy_info payload: {}",
                outcome.value
            );
        }
        assert!(outcome
            .logs
            .iter()
            .any(|l| l.contains("security_policy_info computed")));
    }

    #[test]
    fn security_policy_info_matches_default_policy_values() {
        let outcome = security_policy_info();
        let default = SecurityPolicy::default();
        assert_eq!(outcome.value["autonomy"], json!(default.autonomy));
        assert_eq!(
            outcome.value["allowed_commands"],
            json!(default.allowed_commands)
        );
        assert_eq!(
            outcome.value["max_actions_per_hour"],
            json!(default.max_actions_per_hour)
        );
        assert_eq!(
            outcome.value["workspace_only"],
            json!(default.workspace_only)
        );
        assert_eq!(
            outcome.value["block_high_risk_commands"],
            json!(default.block_high_risk_commands)
        );
        assert_eq!(
            outcome.value["require_approval_for_medium_risk"],
            json!(default.require_approval_for_medium_risk)
        );
    }

    #[test]
    fn client_sessions_status_reports_static_bearer_limitations() {
        let outcome = client_sessions_status();
        assert_eq!(
            outcome.value["session_model"],
            json!("static_bearer_plus_device_sessions")
        );
        assert_eq!(outcome.value["device_scoped_tokens"], json!(true));
        assert_eq!(outcome.value["revocation_supported"], json!(true));
        assert_eq!(outcome.value["static_bearer_enabled"], json!(true));
        assert_eq!(outcome.value["provider_tokens_server_side"], json!(true));
        assert_eq!(outcome.value["mobile_public_ready"], json!(false));
        assert!(outcome.value["sessions"].is_object());
        assert!(outcome
            .logs
            .iter()
            .any(|l| l.contains("security_client_sessions_status computed")));
    }

    #[test]
    fn client_sessions_revoke_rejects_empty_id() {
        let outcome = client_sessions_revoke(" ");

        assert_eq!(outcome.value["revoked"], json!(false));
        assert!(outcome.value["error"]
            .as_str()
            .unwrap()
            .contains("session_id"));
    }
}
