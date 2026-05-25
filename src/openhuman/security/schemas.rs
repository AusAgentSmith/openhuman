use serde_json::{Map, Value};

use crate::core::all::{ControllerFuture, RegisteredController};
use crate::core::{ControllerSchema, FieldSchema, TypeSchema};
use crate::rpc::RpcOutcome;

pub fn all_controller_schemas() -> Vec<ControllerSchema> {
    vec![
        schemas("policy_info"),
        schemas("client_sessions_status"),
        schemas("client_sessions_create"),
        schemas("client_sessions_list"),
        schemas("client_sessions_revoke"),
    ]
}

pub fn all_registered_controllers() -> Vec<RegisteredController> {
    vec![
        RegisteredController {
            schema: schemas("policy_info"),
            handler: handle_policy_info,
        },
        RegisteredController {
            schema: schemas("client_sessions_status"),
            handler: handle_client_sessions_status,
        },
        RegisteredController {
            schema: schemas("client_sessions_create"),
            handler: handle_client_sessions_create,
        },
        RegisteredController {
            schema: schemas("client_sessions_list"),
            handler: handle_client_sessions_list,
        },
        RegisteredController {
            schema: schemas("client_sessions_revoke"),
            handler: handle_client_sessions_revoke,
        },
    ]
}

pub fn schemas(function: &str) -> ControllerSchema {
    match function {
        "policy_info" => ControllerSchema {
            namespace: "security",
            function: "policy_info",
            description: "Return the active security/autonomy policy used by the core runtime.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "policy",
                ty: TypeSchema::Json,
                comment: "Security policy metadata and feature flags.",
                required: true,
            }],
        },
        "client_sessions_status" => ControllerSchema {
            namespace: "security",
            function: "client_sessions_status",
            description: "Return the current client/session token model and whether mobile-safe device tokens and revocation are implemented.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "status",
                ty: TypeSchema::Json,
                comment: "Client/session token readiness metadata for desktop/mobile clients.",
                required: true,
            }],
        },
        "client_sessions_create" => ControllerSchema {
            namespace: "security",
            function: "client_sessions_create",
            description: "Create a named device-scoped client session token. The plaintext token is returned once and only its hash is stored.",
            inputs: vec![FieldSchema {
                name: "label",
                ty: TypeSchema::Option(Box::new(TypeSchema::String)),
                comment: "Human-readable client/device label.",
                required: false,
            }],
            outputs: vec![FieldSchema {
                name: "session",
                ty: TypeSchema::Json,
                comment: "Created session metadata plus one-time plaintext token.",
                required: true,
            }],
        },
        "client_sessions_list" => ControllerSchema {
            namespace: "security",
            function: "client_sessions_list",
            description: "List client sessions without exposing token hashes or plaintext tokens.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "sessions",
                ty: TypeSchema::Json,
                comment: "Client session metadata.",
                required: true,
            }],
        },
        "client_sessions_revoke" => ControllerSchema {
            namespace: "security",
            function: "client_sessions_revoke",
            description: "Revoke a client session so its bearer token can no longer authenticate RPC requests.",
            inputs: vec![FieldSchema {
                name: "session_id",
                ty: TypeSchema::String,
                comment: "Client session id to revoke.",
                required: true,
            }],
            outputs: vec![FieldSchema {
                name: "result",
                ty: TypeSchema::Json,
                comment: "Revocation result.",
                required: true,
            }],
        },
        _ => ControllerSchema {
            namespace: "security",
            function: "unknown",
            description: "Unknown security controller function.",
            inputs: vec![],
            outputs: vec![],
        },
    }
}

fn handle_policy_info(_params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async { to_json(crate::openhuman::security::rpc::security_policy_info()) })
}

fn handle_client_sessions_status(_params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async { to_json(crate::openhuman::security::rpc::client_sessions_status()) })
}

fn handle_client_sessions_create(params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let label = params.get("label").and_then(Value::as_str);
        to_json(crate::openhuman::security::rpc::client_sessions_create(
            label,
        ))
    })
}

fn handle_client_sessions_list(_params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async { to_json(crate::openhuman::security::rpc::client_sessions_list()) })
}

fn handle_client_sessions_revoke(params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let session_id = params
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        to_json(crate::openhuman::security::rpc::client_sessions_revoke(
            session_id,
        ))
    })
}

fn to_json<T: serde::Serialize>(outcome: RpcOutcome<T>) -> Result<Value, String> {
    outcome.into_cli_compatible_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_cover_registered_controllers() {
        let schemas = all_controller_schemas();
        let controllers = all_registered_controllers();

        assert_eq!(schemas.len(), 5);
        assert_eq!(schemas.len(), controllers.len());
        assert!(schemas.iter().any(|schema| {
            schema.namespace == "security" && schema.function == "client_sessions_status"
        }));
        assert!(schemas.iter().any(|schema| {
            schema.namespace == "security" && schema.function == "client_sessions_revoke"
        }));
    }

    #[test]
    fn client_sessions_status_schema_is_no_arg() {
        let schema = schemas("client_sessions_status");

        assert_eq!(schema.namespace, "security");
        assert_eq!(schema.function, "client_sessions_status");
        assert!(schema.inputs.is_empty());
    }

    #[test]
    fn client_sessions_create_accepts_optional_label() {
        let schema = schemas("client_sessions_create");

        assert_eq!(schema.namespace, "security");
        assert_eq!(schema.function, "client_sessions_create");
        assert_eq!(schema.inputs.len(), 1);
        assert!(!schema.inputs[0].required);
    }
}
