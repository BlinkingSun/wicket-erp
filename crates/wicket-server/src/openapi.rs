//! OpenAPI document generated from the capability table (ADR 0010 / T-25).
//!
//! T-44 (`just openapi-fixture` / `lint-openapi-fixture`) gates the `(method, path)`
//! set against `tests/fixtures/openapi-operations.txt`. That fixture does not
//! carry request/response schemas. Wave 1 schemas are derived from the Rust
//! types the handlers serialize (`schemars`, ADR 0011) and attached here by
//! capability id. Presence is gated by the table-walk test
//! `every_in_scope_capability_has_a_schema_binding` and by served-document
//! assertions in `tests/slice.rs`. There is no schema
//! fixture to regenerate: change a type, serve the document, and the tests
//! read what the binary actually emits. Verify with `just ci` and `just ci-db`.

use schemars::JsonSchema;
use serde_json::{Value, json};
use wicket_jobs::JobStatus;
use wicket_mod_genealogy::{AcceptedBody, Impact, TraceBody};
use wicket_mod_items::api::ItemBody;
use wicket_mod_lots::{LotBody, SerialBody};
use wicket_module::SignatureEdge;

use crate::boot::AppState;
use crate::capabilities::{self, Capability};
use crate::envelope::ListBody;
use crate::handlers::{
    ItemPatch, LoginBody, LoginResponse, NavigationBody, OnHandBody, identity::PrincipalBody,
};
use crate::wire::MoneyBody;

/// Capability ids whose request/response types this wave schemas. Twin of the
/// `schema_binding` match; a deleted arm still compiles, so the table-walk test
/// is the guarantee (ADR 0011 amendment).
#[cfg_attr(not(test), allow(dead_code))]
const SCHEMA_CAPABILITIES: &[&str] = &[
    "getItem",
    "listItems",
    "updateItem",
    "getLot",
    "listSerials",
    "traceGenealogy",
    "getImpact",
    "getGenealogyJob",
    "getOnHand",
    "login",
    "getNavigation",
    "getOwnProfile",
];

struct SchemaBinding {
    request: Option<Value>,
    response: Value,
}

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
            "schemas": component_schemas()
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
    attach_body_schemas(&mut op_v, cap.id);
    entry[cap.method.to_ascii_lowercase()] = op_v;
}

fn attach_body_schemas(op_v: &mut Value, id: &str) {
    let Some(binding) = schema_binding(id) else {
        return;
    };
    op_v["responses"]["200"]["content"] = json!({
        "application/json": { "schema": binding.response }
    });
    if let Some(request) = binding.request {
        op_v["requestBody"] = json!({
            "required": true,
            "content": {
                "application/json": { "schema": request }
            }
        });
    }
}

fn schema_ref<T: JsonSchema>() -> Value {
    json!({ "$ref": format!("#/components/schemas/{}", T::schema_name()) })
}

fn schema_binding(id: &str) -> Option<SchemaBinding> {
    match id {
        "getItem" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<ItemBody>(),
        }),
        "listItems" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<ItemBody>>(),
        }),
        "updateItem" => Some(SchemaBinding {
            request: Some(schema_ref::<ItemPatch>()),
            response: schema_ref::<ItemBody>(),
        }),
        "getLot" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<LotBody>(),
        }),
        "listSerials" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<SerialBody>>(),
        }),
        "traceGenealogy" => Some(SchemaBinding {
            request: None,
            response: json!({
                "oneOf": [
                    schema_ref::<TraceBody>(),
                    schema_ref::<AcceptedBody>(),
                ]
            }),
        }),
        "getImpact" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<Impact>(),
        }),
        "getGenealogyJob" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<JobStatus>(),
        }),
        "getOnHand" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<OnHandBody>(),
        }),
        "login" => Some(SchemaBinding {
            request: Some(schema_ref::<LoginBody>()),
            response: schema_ref::<LoginResponse>(),
        }),
        "getNavigation" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<NavigationBody>(),
        }),
        "getOwnProfile" => Some(SchemaBinding {
            request: None,
            response: schema_ref::<PrincipalBody>(),
        }),
        _ => None,
    }
}

fn error_envelope_schema() -> Value {
    json!({
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
    })
}

fn component_schemas() -> serde_json::Map<String, Value> {
    let mut schemas = serde_json::Map::new();
    schemas.insert("ErrorEnvelope".to_owned(), error_envelope_schema());
    merge_type::<ItemBody>(&mut schemas);
    merge_type::<ListBody<ItemBody>>(&mut schemas);
    merge_type::<ItemPatch>(&mut schemas);
    merge_type::<LotBody>(&mut schemas);
    merge_type::<SerialBody>(&mut schemas);
    merge_type::<ListBody<SerialBody>>(&mut schemas);
    merge_type::<TraceBody>(&mut schemas);
    merge_type::<AcceptedBody>(&mut schemas);
    merge_type::<Impact>(&mut schemas);
    merge_type::<JobStatus>(&mut schemas);
    merge_type::<OnHandBody>(&mut schemas);
    merge_type::<LoginBody>(&mut schemas);
    merge_type::<LoginResponse>(&mut schemas);
    merge_type::<NavigationBody>(&mut schemas);
    merge_type::<PrincipalBody>(&mut schemas);
    merge_type::<MoneyBody>(&mut schemas);
    merge_type::<wicket_core::AnyQuantity>(&mut schemas);
    merge_type::<wicket_core::MoneyWire>(&mut schemas);
    convert_trace_body_anyof_to_oneof(&mut schemas);
    schemas
}

fn merge_type<T: JsonSchema>(into: &mut serde_json::Map<String, Value>) {
    let root = schemars::schema_for!(T);
    let mut value = serde_json::to_value(root).expect("schema json");
    rewrite_definition_refs(&mut value);
    let defs = value
        .as_object_mut()
        .and_then(|o| o.remove("definitions").or_else(|| o.remove("$defs")));
    if let Some(Value::Object(defs)) = defs {
        for (k, v) in defs {
            into.entry(k).or_insert(v);
        }
    }
    if let Some(obj) = value.as_object_mut() {
        obj.remove("$schema");
        if obj.get("$ref").is_none() && !obj.is_empty() {
            into.entry(T::schema_name()).or_insert(value);
        }
    }
}

fn rewrite_definition_refs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get_mut("$ref") {
                if let Some(rest) = r.strip_prefix("#/definitions/") {
                    *r = format!("#/components/schemas/{rest}");
                } else if let Some(rest) = r.strip_prefix("#/$defs/") {
                    *r = format!("#/components/schemas/{rest}");
                }
            }
            for v in map.values_mut() {
                rewrite_definition_refs(v);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                rewrite_definition_refs(v);
            }
        }
        _ => {}
    }
}

fn convert_trace_body_anyof_to_oneof(schemas: &mut serde_json::Map<String, Value>) {
    let trace = schemas
        .get_mut(&TraceBody::schema_name())
        .unwrap_or_else(|| panic!("{} missing from components", TraceBody::schema_name()));
    let obj = trace
        .as_object_mut()
        .unwrap_or_else(|| panic!("{} schema is not an object", TraceBody::schema_name()));
    if let Some(any) = obj.remove("anyOf") {
        obj.insert("oneOf".to_owned(), any);
    } else {
        assert!(
            obj.contains_key("oneOf"),
            "{} schema has neither anyOf nor oneOf",
            TraceBody::schema_name()
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities;

    #[test]
    fn every_in_scope_capability_has_a_schema_binding() {
        let table: Vec<_> = capabilities::table().map(|c| c.id).collect();
        for id in SCHEMA_CAPABILITIES {
            assert!(
                table.contains(id),
                "SCHEMA_CAPABILITIES names unknown capability {id}"
            );
            assert!(schema_binding(id).is_some(), "no schema binding for {id}");
        }
        assert_eq!(
            SCHEMA_CAPABILITIES.len(),
            12,
            "Wave 1 in-scope set is the twelve named in SPEC §1"
        );
    }
}
