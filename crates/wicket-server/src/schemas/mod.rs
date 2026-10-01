//! Per-area OpenAPI body-schema registration (ADR 0011).
//!
//! Later lanes add bindings only in their area module. This file pre-declares
//! every area so a later lane never edits `mod.rs`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde_json::map::Entry;
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
            insert_schema(into, k, v);
        }
    }
    if let Some(obj) = value.as_object_mut() {
        obj.remove("$schema");
        if obj.get("$ref").is_none() && !obj.is_empty() {
            insert_schema(into, T::schema_name(), value);
        }
    }
}

/// Keep the first object when the shape matches. A different shape is a hard error.
///
/// Shape drops every `description` key. It also drops a `title` key on a schema
/// object (`type`, `properties`, `$ref`, combinators, or `items` is present).
/// `ListBody_for_SerialBody` differs only by description. The same Rust type is
/// inserted again from a parent definition, and schemars omits `title` on that
/// copy. A property named `title` stays in the shape. The first object is never
/// rewritten.
fn insert_schema(into: &mut serde_json::Map<String, Value>, name: String, incoming: Value) {
    match into.entry(name) {
        Entry::Vacant(slot) => {
            slot.insert(incoming);
        }
        Entry::Occupied(existing) => {
            if schema_shape(existing.get()) != schema_shape(&incoming) {
                panic!(
                    "OpenAPI schema `{}` re-registered with a different shape",
                    existing.key()
                );
            }
        }
    }
}

fn schema_shape(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let annotation = is_schema_object(map);
            let mut out = serde_json::Map::new();
            for (key, child) in map {
                if key == "description" || (key == "title" && annotation) {
                    continue;
                }
                out.insert(key.clone(), schema_shape(child));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(schema_shape).collect()),
        other => other.clone(),
    }
}

fn is_schema_object(map: &serde_json::Map<String, Value>) -> bool {
    map.contains_key("type")
        || map.contains_key("properties")
        || map.contains_key("$ref")
        || map.contains_key("oneOf")
        || map.contains_key("anyOf")
        || map.contains_key("allOf")
        || map.contains_key("enum")
        || map.contains_key("items")
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

#[cfg(test)]
mod tests {
    use super::merge_type;
    use schemars::JsonSchema;
    use serde_json::Value;

    fn serial_list_name() -> String {
        <crate::envelope::ListBody<wicket_mod_lots::SerialBody> as JsonSchema>::schema_name()
    }

    #[test]
    fn list_body_for_serial_body_shapes_match_and_envelope_text_wins() {
        let name = serial_list_name();
        assert_eq!(
            <wicket_mod_lots::ListBody<wicket_mod_lots::SerialBody> as JsonSchema>::schema_name(),
            name,
            "the legal collision must share one schema name"
        );
        let mut schemas = serde_json::Map::new();
        merge_type::<crate::envelope::ListBody<wicket_mod_lots::SerialBody>>(&mut schemas);
        let first = schemas.get(&name).expect("envelope list schema").clone();
        let description = first
            .get("description")
            .and_then(Value::as_str)
            .expect("envelope description");
        assert!(
            description.contains("docs/10"),
            "envelope description must be the kept text: {description}"
        );
        merge_type::<wicket_mod_lots::ListBody<wicket_mod_lots::SerialBody>>(&mut schemas);
        assert_eq!(
            schemas.get(&name).expect("kept list schema"),
            &first,
            "shape-equal re-registration must keep the first object unchanged"
        );
    }

    #[test]
    fn identical_double_merge_of_the_same_type_does_not_fail() {
        let mut schemas = serde_json::Map::new();
        merge_type::<crate::envelope::ListBody<wicket_mod_lots::SerialBody>>(&mut schemas);
        let first = schemas.clone();
        merge_type::<crate::envelope::ListBody<wicket_mod_lots::SerialBody>>(&mut schemas);
        assert_eq!(schemas, first);
    }

    #[test]
    #[should_panic(expected = "re-registered with a different shape")]
    fn different_shape_under_an_existing_name_panics() {
        let name = serial_list_name();
        let mut schemas = serde_json::Map::new();
        schemas.insert(name, serde_json::json!({ "type": "string" }));
        merge_type::<crate::envelope::ListBody<wicket_mod_lots::SerialBody>>(&mut schemas);
    }

    #[test]
    fn schema_title_annotation_keeps_the_first_object() {
        let mut schemas = serde_json::Map::new();
        let first = serde_json::json!({
            "title": "ItemBody",
            "description": "root",
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Surrogate id." }
            }
        });
        super::insert_schema(&mut schemas, "ItemBody".to_owned(), first.clone());
        super::insert_schema(
            &mut schemas,
            "ItemBody".to_owned(),
            serde_json::json!({
                "description": "definition copy",
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "other words" }
                }
            }),
        );
        assert_eq!(schemas.get("ItemBody"), Some(&first));
    }

    #[test]
    #[should_panic(expected = "re-registered with a different shape")]
    fn title_property_type_change_is_a_different_shape() {
        let mut schemas = serde_json::Map::new();
        super::insert_schema(
            &mut schemas,
            "Doc".to_owned(),
            serde_json::json!({
                "type": "object",
                "properties": { "title": { "type": "string" } }
            }),
        );
        super::insert_schema(
            &mut schemas,
            "Doc".to_owned(),
            serde_json::json!({
                "type": "object",
                "properties": { "title": { "type": "integer" } }
            }),
        );
    }

    #[test]
    fn component_schemas_match_committed_fixtures_and_keep_envelope_text() {
        let schemas = super::component_schemas();
        let description = schemas
            .get(&serial_list_name())
            .and_then(|schema| schema.get("description"))
            .and_then(Value::as_str)
            .expect("ListBody_for_SerialBody description");
        assert!(
            description.contains("docs/10"),
            "production merge must keep the envelope text: {description}"
        );
        for fixture in [
            include_str!("../../tests/fixtures/openapi-document.json"),
            include_str!("../../tests/fixtures/openapi-document-regulated.json"),
        ] {
            let doc: Value = serde_json::from_str(fixture).expect("openapi fixture");
            let expected = doc["components"]["schemas"]
                .as_object()
                .expect("components.schemas");
            assert_eq!(&schemas, expected);
        }
    }
}
