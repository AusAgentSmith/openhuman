//! Server-runtime capability inventory for controller and feature gating.

pub mod ops;
mod schemas;
mod types;

pub use ops::{capability_for, inventory, status};
pub use schemas::{
    all_controller_schemas as all_capabilities_controller_schemas,
    all_registered_controllers as all_capabilities_registered_controllers,
};
pub use types::{
    CapabilityInventory, CapabilityLabel, CapabilityStatus, ControllerCapability,
    ControllerCapabilityEntry, ControllerVisibility, RuntimeDependencyStatus,
};
