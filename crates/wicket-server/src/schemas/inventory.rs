//! Body schemas for inventory operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::OnHandBody;
use serde_json::Value;

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getOnHand",
        SchemaBinding {
            request: None,
            response: schema_ref::<OnHandBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<OnHandBody>(schemas);
}
