//! Body schemas for kernel operations (login, navigation, OpenAPI, manifest, audit).
//!
//! `health` returns `text/plain` and `logout` returns 204 with no body. They are
//! not JSON-200 bindings. [`apply_non_json_success`] writes those success
//! responses when the document is built.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::kernel::{
    AuditExportBody, AuditHeadBody, ManifestModuleBody, OpenApiDocument, SignatureEdgeBody,
    ValidationManifestBody,
};
use crate::handlers::{LoginBody, LoginResponse, NavigationBody};
use serde_json::{Value, json};

/// Write the success response for operations that are not JSON on HTTP 200.
///
/// `health` is `200` `text/plain` with a string schema and no `application/json`.
/// `logout` is `204` with a description and no body; the shared `200` stamp is
/// removed and no `requestBody` is added. Returns whether `id` is one of those
/// operations.
pub fn apply_non_json_success(op: &mut Value, id: &str) -> bool {
    match id {
        "health" => {
            op["responses"]["200"]["content"] = json!({
                "text/plain": {
                    "schema": { "type": "string" }
                }
            });
            true
        }
        "logout" => {
            if let Some(responses) = op.get_mut("responses").and_then(Value::as_object_mut) {
                responses.remove("200");
                responses.insert("204".to_owned(), json!({ "description": "no content" }));
            }
            true
        }
        _ => false,
    }
}

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "login",
        SchemaBinding {
            request: Some(schema_ref::<LoginBody>()),
            response: schema_ref::<LoginResponse>(),
        },
    );
    map.insert(
        "getNavigation",
        SchemaBinding {
            request: None,
            response: schema_ref::<NavigationBody>(),
        },
    );
    map.insert(
        "getOpenApi",
        SchemaBinding {
            request: None,
            response: schema_ref::<OpenApiDocument>(),
        },
    );
    map.insert(
        "getValidationManifest",
        SchemaBinding {
            request: None,
            response: schema_ref::<ValidationManifestBody>(),
        },
    );
    map.insert(
        "exportAudit",
        SchemaBinding {
            request: None,
            response: schema_ref::<AuditExportBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LoginBody>(schemas);
    merge_type::<LoginResponse>(schemas);
    merge_type::<NavigationBody>(schemas);
    merge_type::<OpenApiDocument>(schemas);
    merge_type::<ValidationManifestBody>(schemas);
    merge_type::<ManifestModuleBody>(schemas);
    merge_type::<SignatureEdgeBody>(schemas);
    merge_type::<AuditExportBody>(schemas);
    merge_type::<AuditHeadBody>(schemas);
}
