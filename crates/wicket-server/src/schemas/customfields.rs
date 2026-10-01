//! Body schemas for custom-field operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::customfields::{
    DefineBody, DefineResponse, FieldWrite, RetireResponse, SetBody,
};
use serde_json::Value;
use wicket_customfields::{Definition, ValueWire};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "defineCustomField",
        SchemaBinding {
            request: Some(schema_ref::<DefineBody>()),
            response: schema_ref::<DefineResponse>(),
        },
    );
    map.insert(
        "listCustomFieldDefinitions",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<Definition>>(),
        },
    );
    map.insert(
        "retireCustomField",
        SchemaBinding {
            request: None,
            response: schema_ref::<RetireResponse>(),
        },
    );
    map.insert(
        "setItemCustomFields",
        SchemaBinding {
            request: Some(schema_ref::<SetBody>()),
            response: schema_ref::<ListBody<ValueWire>>(),
        },
    );
    map.insert(
        "getItemCustomFields",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<ValueWire>>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<DefineBody>(schemas);
    merge_type::<DefineResponse>(schemas);
    merge_type::<Definition>(schemas);
    merge_type::<ListBody<Definition>>(schemas);
    merge_type::<RetireResponse>(schemas);
    merge_type::<SetBody>(schemas);
    merge_type::<FieldWrite>(schemas);
    merge_type::<ValueWire>(schemas);
    merge_type::<ListBody<ValueWire>>(schemas);
}

#[cfg(test)]
mod refs {
    use std::collections::HashSet;

    use serde_json::{Map, Value};

    use super::super::SchemaMap;
    use super::super::component_schemas;
    use super::register;

    const OPS: &[&str] = &[
        "defineCustomField",
        "listCustomFieldDefinitions",
        "retireCustomField",
        "setItemCustomFields",
        "getItemCustomFields",
    ];

    fn resolve<'a>(schemas: &'a Map<String, Value>, mut schema: &'a Value) -> &'a Value {
        for _ in 0..8 {
            if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
                let name = reference.rsplit('/').next().expect("ref name");
                schema = &schemas[name];
                continue;
            }
            if let Some(all) = schema.get("allOf").and_then(Value::as_array)
                && all.len() == 1
            {
                schema = &all[0];
                continue;
            }
            break;
        }
        schema
    }

    fn string_enum_tokens(schema: &Value) -> Vec<&str> {
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            return values
                .iter()
                .map(|item| item.as_str().expect("enum token"))
                .collect();
        }
        schema["oneOf"]
            .as_array()
            .unwrap_or_else(|| panic!("{schema}"))
            .iter()
            .map(|branch| branch["enum"][0].as_str().expect("enum token"))
            .collect()
    }

    fn walk(value: &Value, schemas: &Map<String, Value>, seen: &mut HashSet<String>) {
        if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
            let name = reference.rsplit('/').next().expect("ref name");
            assert!(schemas.contains_key(name), "dangling $ref {reference}");
            if seen.insert(name.to_owned()) {
                walk(&schemas[name], schemas, seen);
            }
        }
        match value {
            Value::Object(map) => {
                for child in map.values() {
                    walk(child, schemas, seen);
                }
            }
            Value::Array(items) => {
                for child in items {
                    walk(child, schemas, seen);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn customfield_bindings_resolve_and_describe_the_wire() {
        let mut map = SchemaMap::new();
        register(&mut map);
        let schemas = component_schemas();
        assert_eq!(map.len(), OPS.len());
        for id in OPS {
            let binding = map.get(id).unwrap_or_else(|| panic!("missing {id}"));
            let mut seen = HashSet::new();
            walk(&binding.response, &schemas, &mut seen);
            if let Some(request) = &binding.request {
                walk(request, &schemas, &mut seen);
            }
        }
        assert!(map["listCustomFieldDefinitions"].request.is_none());
        assert!(map["retireCustomField"].request.is_none());
        assert!(map["getItemCustomFields"].request.is_none());
        assert!(map["defineCustomField"].request.is_some());
        assert!(map["setItemCustomFields"].request.is_some());

        let field_type = string_enum_tokens(&schemas["FieldType"]);
        assert_eq!(field_type[0], "String", "{field_type:?}");
        assert!(field_type.iter().all(|v| *v != "string"), "{field_type:?}");
        assert_eq!(
            string_enum_tokens(&schemas["DefinitionStatus"]),
            ["Active", "Retired"]
        );
        assert_eq!(
            schemas["ValueWire"]["properties"]["type"]["enum"][0],
            "string"
        );
        assert!(schemas["ValueWire"]["properties"]["value"]["oneOf"].is_array());
        assert_eq!(
            schemas["RetireResponse"]["properties"]["status"]["enum"],
            serde_json::json!(["retired"])
        );
        assert_eq!(
            schemas["DefineResponse"]["properties"]["id"]["format"],
            "uuid"
        );
        assert_eq!(
            schemas["DefineBody"]["properties"]["type"]["enum"][0],
            "string"
        );
        let definition_id = resolve(&schemas, &schemas["Definition"]["properties"]["id"]);
        assert_eq!(definition_id["type"], "string", "{definition_id}");
        assert_eq!(definition_id["format"], "uuid", "{definition_id}");
        assert_ne!(
            schemas["DefineBody"]["properties"]["type"]["enum"][0],
            "String"
        );
    }
}
