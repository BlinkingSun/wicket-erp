//! OpenAPI document generated from the capability table (ADR 0010 / T-25).

use serde_json::{Value, json};
use wicket_module::SignatureEdge;

use crate::boot::AppState;
use crate::capabilities::{self, Capability};

/// Merge the capability table into one OpenAPI 3 document.
pub fn document(state: &AppState) -> Value {
    let edges = &state.kernel().profile.signature_edges;
    let mut paths = serde_json::Map::new();
    for cap in capabilities::table() {
        insert(&mut paths, cap, required_signature(cap, edges));
    }
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Wicket HTTP API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Wave 2s slice. The UI consumes this document (ADR 0009)."
        },
        "paths": paths,
        "components": {
            "schemas": {
                "ErrorEnvelope": {
                    "type": "object",
                    "required": ["error"],
                    "properties": {
                        "error": {
                            "type": "object",
                            "required": ["code", "message", "request_id"],
                            "properties": {
                                "code": { "type": "string" },
                                "message": { "type": "string" },
                                "field": { "type": ["string", "null"] },
                                "request_id": { "type": "string", "format": "uuid" }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn required_signature<'a>(
    cap: &Capability,
    edges: &'a [SignatureEdge],
) -> Option<(&'a str, &'a str)> {
    // One handler serves release/hold/reject; a single stamped meaning is false.
    if cap.id == "setLotStatus" {
        return None;
    }
    let doc_type = cap.doc_type?;
    let edge = cap.edge?;
    edges.iter().find_map(|declared| match declared {
        SignatureEdge::Required {
            module,
            edge: name,
            meaning,
            permission,
        } if module == doc_type && name == edge => Some((meaning.as_str(), permission.as_str())),
        _ => None,
    })
}

fn insert(
    paths: &mut serde_json::Map<String, Value>,
    cap: &Capability,
    signature: Option<(&str, &str)>,
) {
    let entry = paths
        .entry(cap.path.to_string())
        .or_insert_with(|| json!({}));
    let mut op_v = json!({
        "operationId": cap.id,
        "x-wicket-permission": cap.permission,
        "responses": {
            "200": { "description": "ok" },
            "201": { "description": "created" },
            "400": { "description": "validation" },
            "401": { "description": "unauthenticated" },
            "403": { "description": "forbidden" },
            "404": { "description": "not found" },
            "409": { "description": "conflict" }
        }
    });
    let mut parameters = path_parameters(cap.path);
    parameters.extend(query_parameters(cap.id));
    if requires_idempotency_key(cap.id) {
        parameters.push(header_param(
            "Idempotency-Key",
            json!({ "type": "string", "format": "uuid" }),
        ));
    }
    if requires_if_match(cap.id) {
        parameters.push(header_param("If-Match", json!({ "type": "string" })));
    }
    if let Some((meaning, permission)) = signature {
        parameters.push(header_param(
            "X-Wicket-Signature",
            json!({ "type": "string", "format": "uuid" }),
        ));
        op_v["x-wicket-signature"] = json!({
            "meaning": meaning,
            "permission": permission
        });
    }
    if !parameters.is_empty() {
        op_v["parameters"] = Value::Array(parameters);
    }
    entry[cap.method.to_ascii_lowercase()] = op_v;
}

fn path_placeholders(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('}') else {
            break;
        };
        out.push(&rest[..end]);
        rest = &rest[end + 1..];
    }
    out
}

fn path_parameters(path: &str) -> Vec<Value> {
    path_placeholders(path)
        .into_iter()
        .map(|name| {
            json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": "string", "format": "uuid" }
            })
        })
        .collect()
}

fn query_param(name: &str, required: bool, schema: Value) -> Value {
    json!({
        "name": name,
        "in": "query",
        "required": required,
        "schema": schema
    })
}

fn limit_schema() -> Value {
    json!({
        "type": "integer",
        "minimum": 1,
        "maximum": 200,
        "default": 50
    })
}

fn uuid_schema() -> Value {
    json!({ "type": "string", "format": "uuid" })
}

fn string_schema() -> Value {
    json!({ "type": "string" })
}

fn query_parameters(id: &str) -> Vec<Value> {
    match id {
        "listItems" => vec![
            query_param("limit", false, limit_schema()),
            query_param("cursor", false, string_schema()),
            query_param("kind", false, string_schema()),
            query_param("status", false, string_schema()),
            query_param("number_prefix", false, string_schema()),
        ],
        "listLocations" | "listLots" => vec![
            query_param("limit", false, limit_schema()),
            query_param("cursor", false, string_schema()),
        ],
        "listLocationTree" => vec![query_param(
            "include_inactive",
            false,
            json!({ "type": "boolean", "default": false }),
        )],
        "listWorkOrders" => vec![
            query_param("limit", false, limit_schema()),
            query_param("cursor", false, string_schema()),
            query_param("status", false, string_schema()),
        ],
        "getOnHand" => vec![
            query_param("item_id", true, uuid_schema()),
            query_param("location_id", false, uuid_schema()),
            query_param("lot_id", false, uuid_schema()),
        ],
        "traceGenealogy" => vec![
            query_param("from_lot_id", true, uuid_schema()),
            query_param(
                "direction",
                false,
                json!({
                    "type": "string",
                    "default": "forward",
                    "enum": ["forward", "backward", "both"]
                }),
            ),
        ],
        "listCustomFieldDefinitions" => vec![query_param("entity", true, string_schema())],
        _ => Vec::new(),
    }
}

fn header_param(name: &str, schema: Value) -> Value {
    json!({
        "name": name,
        "in": "header",
        "required": true,
        "schema": schema
    })
}

/// Per-route: handler calls `idempotency::require_key`. Not "all POST".
fn requires_idempotency_key(id: &str) -> bool {
    matches!(
        id,
        "login"
            | "logout"
            | "createPrincipal"
            | "renamePrincipal"
            | "deactivatePrincipal"
            | "resetLoginCredential"
            | "changeOwnLoginCredential"
            | "setOwnSigningCredential"
            | "approveCalibration"
            | "esignMint"
            | "defineCustomField"
            | "retireCustomField"
            | "setItemCustomFields"
            | "createDocument"
            | "createDocumentRevision"
            | "submitDocument"
            | "approveDocument"
            | "renderPrint"
            | "archivePrint"
            | "releaseFromQuarantine"
            | "reverseIssue"
            | "createItem"
            | "updateItem"
            | "releaseItem"
            | "createLocation"
            | "deactivateLocation"
            | "createLot"
            | "setLotStatus"
            | "createPackage"
            | "createSerials"
            | "createReceipt"
            | "createCount"
            | "createWorkOrder"
            | "releaseWorkOrder"
            | "issueWorkOrder"
            | "completeWorkOrder"
    )
}

/// Per-route: handler calls `require_if_match`. Not "all Transition".
fn requires_if_match(id: &str) -> bool {
    matches!(
        id,
        "updateItem"
            | "releaseItem"
            | "deactivateLocation"
            | "setLotStatus"
            | "releaseFromQuarantine"
            | "releaseWorkOrder"
            | "issueWorkOrder"
            | "completeWorkOrder"
            | "approveCalibration"
            | "retireCustomField"
            | "submitDocument"
            | "approveDocument"
    )
}

/// Every path+method in a served OpenAPI document.
pub fn registered_operations(doc: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
        for (path, item) in paths {
            if let Some(obj) = item.as_object() {
                for method in obj.keys() {
                    if matches!(method.as_str(), "get" | "post" | "patch" | "put" | "delete") {
                        out.push((method.to_ascii_uppercase(), path.clone()));
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// Capability-table operations as (METHOD, path) pairs.
pub fn mounted_operations() -> Vec<(String, String)> {
    capabilities::operations()
}
