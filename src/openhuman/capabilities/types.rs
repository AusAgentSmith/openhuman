use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityLabel {
    ServerSafe,
    ClientOnly,
    DesktopCollector,
    BlockedByTauriBridge,
}

impl CapabilityLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ServerSafe => "server-safe",
            Self::ClientOnly => "client-only",
            Self::DesktopCollector => "desktop-collector",
            Self::BlockedByTauriBridge => "blocked-by-tauri-bridge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerCapability {
    pub label: CapabilityLabel,
    pub mobile_safe: bool,
    pub standalone_server_safe: bool,
    pub requires: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerVisibility {
    Public,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerCapabilityEntry {
    pub method: String,
    pub namespace: String,
    pub function: String,
    pub visibility: ControllerVisibility,
    pub capability: ControllerCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDependencyStatus {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStatus {
    pub counts: BTreeMap<String, usize>,
    pub runtime_dependencies: Vec<RuntimeDependencyStatus>,
    pub blocked_by_tauri_bridge: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityInventory {
    pub controllers: Vec<ControllerCapabilityEntry>,
    pub status: CapabilityStatus,
}
