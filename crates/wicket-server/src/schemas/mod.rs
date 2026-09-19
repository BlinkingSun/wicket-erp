//! Per-area OpenAPI body-schema registration (ADR 0011).
//!
//! Later lanes add bindings only in their area module. This file pre-declares
//! every area so a later lane never edits `mod.rs`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde_json::{Value, json};
use wicket_mod_genealogy::TraceBody;

use crate::handlers::ResolveIdBody;
use crate::wire::MoneyBody;

pub mod customfields;
pub mod documents;
pub mod esign;
pub mod identity;
pub mod inventory;
pub mod items;
pub mod kernel;
pub mod locations;
pub mod lots;
pub mod print;
pub mod production;

/// Request and response body schemas for one capability id.
#[derive(Clone, Debug)]
pub struct SchemaBinding {
    /// Present when the operation advertises a JSON request body.
    pub request: Option<Value>,
    /// JSON Schema (usually a `$ref`) for the 200 response body.
    pub response: Value,
}

/// Capability id → body schemas. BTreeMap so walks are stable.
pub type SchemaMap = BTreeMap<&'static str, SchemaBinding>;

/// Every registered body-schema binding, folded from the area modules.
pub fn all() -> SchemaMap {
    let mut map = SchemaMap::new();
    identity::register(&mut map);
    customfields::register(&mut map);
    documents::register(&mut map);
    print::register(&mut map);
    items::register(&mut map);
    locations::register(&mut map);
    lots::register(&mut map);
    inventory::register(&mut map);
    production::register(&mut map);
    esign::register(&mut map);
    kernel::register(&mut map);
    map
}

/// Binding for `id`, if this wave (or a later area lane) registered one.
pub fn binding(id: &str) -> Option<SchemaBinding> {
    all().get(id).cloned()
}

pub(crate) fn schema_ref<T: JsonSchema>() -> Value {
    json!({ "$ref": format!("#/components/schemas/{}", T::schema_name()) })
}

/// `components.schemas` for the served document.
///
/// Area `merge_components` calls run in the wave-1 insertion order so a
/// pure move of the existing 19 bindings stays byte-identical. Empty areas
/// are no-ops today and the slot later lanes write into.
pub fn component_schemas() -> serde_json::Map<String, Value> {
    let mut schemas = serde_json::Map::new();
    schemas.insert("ErrorEnvelope".to_owned(), error_envelope_schema());
    items::merge_components(&mut schemas);
    lots::merge_components(&mut schemas);
    production::merge_components(&mut schemas);
    inventory::merge_components(&mut schemas);
    kernel::merge_components(&mut schemas);
    identity::merge_components(&mut schemas);
    customfields::merge_components(&mut schemas);
    documents::merge_components(&mut schemas);
    print::merge_components(&mut schemas);
    locations::merge_components(&mut schemas);
    esign::merge_components(&mut schemas);
    merge_type::<ResolveIdBody>(&mut schemas);
    merge_type::<MoneyBody>(&mut schemas);
    merge_type::<wicket_core::AnyQuantity>(&mut schemas);
    merge_type::<wicket_core::MoneyWire>(&mut schemas);
    convert_trace_body_anyof_to_oneof(&mut schemas);
    schemas
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

pub(crate) fn merge_type<T: JsonSchema>(into: &mut serde_json::Map<String, Value>) {
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
