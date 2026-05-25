use serde_json::{Map, Value};

use crate::core::all::{ControllerFuture, RegisteredController};
use crate::core::{ControllerSchema, FieldSchema, TypeSchema};

pub fn all_controller_schemas() -> Vec<ControllerSchema> {
    vec![schemas("inventory"), schemas("status")]
}

pub fn all_registered_controllers() -> Vec<RegisteredController> {
    vec![
        RegisteredController {
            schema: schemas("inventory"),
            handler: handle_inventory,
        },
        RegisteredController {
            schema: schemas("status"),
            handler: handle_status,
        },
    ]
}

pub fn schemas(function: &str) -> ControllerSchema {
    match function {
        "inventory" => ControllerSchema {
            namespace: "capabilities",
            function: "inventory",
            description: "Return every public and internal controller with server-runtime capability labels for desktop/mobile feature gating.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "result",
                ty: TypeSchema::Json,
                comment: "Capability inventory containing controller labels, visibility, runtime dependency status, and bridge-blocked methods.",
                required: true,
            }],
        },
        "status" => ControllerSchema {
            namespace: "capabilities",
            function: "status",
            description: "Return a compact capability summary and runtime dependency status.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "result",
                ty: TypeSchema::Json,
                comment: "Capability counts, runtime dependency status, and bridge-blocked methods.",
                required: true,
            }],
        },
        _ => ControllerSchema {
            namespace: "capabilities",
            function: "unknown",
            description: "Unknown capabilities controller function.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "error",
                ty: TypeSchema::String,
                comment: "Lookup error details.",
                required: true,
            }],
        },
    }
}

fn handle_inventory(_params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move { super::ops::inventory().into_cli_compatible_json() })
}

fn handle_status(_params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move { super::ops::status().into_cli_compatible_json() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_cover_registered_controllers() {
        let schemas = all_controller_schemas();
        let controllers = all_registered_controllers();

        assert_eq!(schemas.len(), 2);
        assert_eq!(controllers.len(), 2);
        assert_eq!(schemas[0].function, controllers[0].schema.function);
        assert_eq!(schemas[1].function, controllers[1].schema.function);
    }

    #[test]
    fn inventory_schema_is_no_arg() {
        let schema = schemas("inventory");
        assert_eq!(schema.namespace, "capabilities");
        assert_eq!(schema.function, "inventory");
        assert!(schema.inputs.is_empty());
    }
}
