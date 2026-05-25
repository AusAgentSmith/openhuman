use std::collections::{BTreeMap, BTreeSet};

use crate::core::all::{self, HttpMethodSchemaDefinition};
use crate::rpc::RpcOutcome;

use super::types::{
    CapabilityInventory, CapabilityLabel, CapabilityStatus, ControllerCapability,
    ControllerCapabilityEntry, ControllerVisibility, RuntimeDependencyStatus,
};

const WEBVIEW_APIS_DEPENDENCY: &str = "tauri:webview_apis";
const WEBVIEW_APIS_PORT_ENV: &str = crate::openhuman::webview_apis::client::PORT_ENV;

pub fn capability_for(namespace: &str, function: &str) -> ControllerCapability {
    classify(namespace, function)
}

pub fn inventory() -> RpcOutcome<CapabilityInventory> {
    let controllers = controller_entries();
    let status = build_status(&controllers);

    RpcOutcome::single_log(
        CapabilityInventory {
            controllers,
            status,
        },
        "capability inventory generated",
    )
}

pub fn status() -> RpcOutcome<CapabilityStatus> {
    let controllers = controller_entries();
    RpcOutcome::single_log(build_status(&controllers), "capability status generated")
}

fn controller_entries() -> Vec<ControllerCapabilityEntry> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();

    for method in all::all_http_method_schemas() {
        let key = method.method.clone();
        seen.insert(key);
        entries.push(entry_for(method, ControllerVisibility::Public));
    }

    for method in all::all_internal_http_method_schemas() {
        if seen.insert(method.method.clone()) {
            entries.push(entry_for(method, ControllerVisibility::Internal));
        }
    }

    entries.sort_by(|a, b| a.method.cmp(&b.method));
    entries
}

fn entry_for(
    method: HttpMethodSchemaDefinition,
    visibility: ControllerVisibility,
) -> ControllerCapabilityEntry {
    ControllerCapabilityEntry {
        capability: classify(method.namespace, method.function),
        method: method.method,
        namespace: method.namespace.to_string(),
        function: method.function.to_string(),
        visibility,
    }
}

fn build_status(controllers: &[ControllerCapabilityEntry]) -> CapabilityStatus {
    let mut counts = BTreeMap::new();
    let mut blocked_by_tauri_bridge = Vec::new();

    for entry in controllers {
        *counts
            .entry(entry.capability.label.as_str().to_string())
            .or_insert(0) += 1;
        if entry.capability.label == CapabilityLabel::BlockedByTauriBridge {
            blocked_by_tauri_bridge.push(entry.method.clone());
        }
    }

    CapabilityStatus {
        counts,
        runtime_dependencies: runtime_dependencies(),
        blocked_by_tauri_bridge,
    }
}

fn classify(namespace: &str, function: &str) -> ControllerCapability {
    match namespace {
        "webview_apis" => blocked_by_tauri_bridge(),
        "notification" if function == "ingest" => desktop_collector(
            "optional-provider-event-source",
            "Ingest path is fed by embedded webviews or equivalent provider event collectors.",
        ),
        "voice"
            if matches!(
                function,
                "server_start" | "server_stop" | "server_status" | "overlay_stt_notify"
            ) =>
        {
            client_only(
                "local-voice-device",
                "Controls the local dictation server, hotkey state, or desktop overlay.",
            )
        }
        "autocomplete" | "meet" | "meet_agent" | "screen_intelligence" | "service"
        | "text_input" => client_only(
            "local-device",
            "Requires local OS, native window, input, capture, or live-call device state.",
        ),
        "connectivity" => client_only(
            "local-core-process",
            "Reports the local sidecar/process diagnostic surface used by desktop boot health.",
        ),
        "http_host" => client_only(
            "local-filesystem-and-network",
            "Hosts a local directory from the running machine over a local HTTP listener.",
        ),
        "provider_surfaces" | "whatsapp_data" => desktop_collector(
            "optional-desktop-collector",
            "Reads server-side state that is populated by an optional desktop collector while the desktop app is running.",
        ),
        _ => server_safe(),
    }
}

fn blocked_by_tauri_bridge() -> ControllerCapability {
    ControllerCapability {
        label: CapabilityLabel::BlockedByTauriBridge,
        mobile_safe: false,
        standalone_server_safe: false,
        requires: vec![
            WEBVIEW_APIS_DEPENDENCY.to_string(),
            format!("env:{WEBVIEW_APIS_PORT_ENV}"),
        ],
        reason: "Proxies through the desktop Tauri webview_apis bridge and live CEF/CDP state."
            .to_string(),
    }
}

fn client_only(requirement: &str, reason: &str) -> ControllerCapability {
    ControllerCapability {
        label: CapabilityLabel::ClientOnly,
        mobile_safe: false,
        standalone_server_safe: false,
        requires: vec![requirement.to_string()],
        reason: reason.to_string(),
    }
}

fn desktop_collector(requirement: &str, reason: &str) -> ControllerCapability {
    ControllerCapability {
        label: CapabilityLabel::DesktopCollector,
        mobile_safe: false,
        standalone_server_safe: true,
        requires: vec![requirement.to_string()],
        reason: reason.to_string(),
    }
}

fn server_safe() -> ControllerCapability {
    ControllerCapability {
        label: CapabilityLabel::ServerSafe,
        mobile_safe: true,
        standalone_server_safe: true,
        requires: Vec::new(),
        reason: "Runs against core-managed server state or direct server-side integrations."
            .to_string(),
    }
}

fn runtime_dependencies() -> Vec<RuntimeDependencyStatus> {
    let webview_bridge_available = std::env::var(WEBVIEW_APIS_PORT_ENV)
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    vec![RuntimeDependencyStatus {
        id: WEBVIEW_APIS_DEPENDENCY.to_string(),
        label: "Desktop webview APIs bridge".to_string(),
        available: webview_bridge_available,
        details: if webview_bridge_available {
            format!("{WEBVIEW_APIS_PORT_ENV} is set for this process")
        } else {
            format!("{WEBVIEW_APIS_PORT_ENV} is not set; desktop CEF/CDP bridge controllers are unavailable")
        },
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_webview_apis_as_blocked_bridge() {
        let capability = capability_for("webview_apis", "gmail_search");

        assert_eq!(capability.label, CapabilityLabel::BlockedByTauriBridge);
        assert!(!capability.mobile_safe);
        assert!(!capability.standalone_server_safe);
        assert!(capability
            .requires
            .iter()
            .any(|requirement| requirement == WEBVIEW_APIS_DEPENDENCY));
    }

    #[test]
    fn inventory_includes_public_and_internal_controllers() {
        let outcome = inventory();
        let methods = outcome
            .value
            .controllers
            .iter()
            .map(|entry| entry.method.as_str())
            .collect::<BTreeSet<_>>();

        assert!(methods.contains("openhuman.health_snapshot"));
        assert!(methods.contains("openhuman.webview_apis_gmail_search"));
        assert!(methods.contains("openhuman.whatsapp_data_ingest"));
    }

    #[test]
    fn status_tracks_blocked_bridge_methods() {
        let outcome = status();

        assert!(outcome
            .value
            .blocked_by_tauri_bridge
            .iter()
            .any(|method| method == "openhuman.webview_apis_gmail_search"));
        assert!(outcome.value.counts.contains_key("server-safe"));
        assert!(outcome.value.counts.contains_key("blocked-by-tauri-bridge"));
    }

    #[test]
    fn classifies_local_device_namespaces_as_client_only() {
        for (namespace, function) in [
            ("connectivity", "diag"),
            ("http_host", "start"),
            ("meet", "status"),
            ("voice", "server_start"),
            ("voice", "overlay_stt_notify"),
        ] {
            let capability = capability_for(namespace, function);
            assert_eq!(
                capability.label,
                CapabilityLabel::ClientOnly,
                "{namespace}.{function}"
            );
            assert!(!capability.mobile_safe, "{namespace}.{function}");
            assert!(!capability.standalone_server_safe, "{namespace}.{function}");
        }
    }

    #[test]
    fn classifies_desktop_collector_backed_namespaces() {
        for (namespace, function) in [
            ("notification", "ingest"),
            ("provider_surfaces", "ingest_event"),
            ("whatsapp_data", "list_chats"),
        ] {
            let capability = capability_for(namespace, function);
            assert_eq!(
                capability.label,
                CapabilityLabel::DesktopCollector,
                "{namespace}.{function}"
            );
            assert!(!capability.mobile_safe, "{namespace}.{function}");
            assert!(capability.standalone_server_safe, "{namespace}.{function}");
        }
    }

    #[test]
    fn leaves_server_processed_voice_calls_server_safe() {
        for function in [
            "status",
            "transcribe_bytes",
            "tts_dispatch",
            "set_providers",
        ] {
            let capability = capability_for("voice", function);
            assert_eq!(capability.label, CapabilityLabel::ServerSafe, "{function}");
            assert!(capability.mobile_safe, "{function}");
            assert!(capability.standalone_server_safe, "{function}");
        }
    }
}
