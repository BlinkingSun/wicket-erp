//! Body schemas for item operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::items::ItemCreate;
use crate::handlers::{ItemPatch, ResolveIdBody};
use serde_json::Value;
use wicket_mod_items::api::ItemBody;

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getItem",
        SchemaBinding {
            request: None,
            response: schema_ref::<ItemBody>(),
        },
    );
    map.insert(
        "listItems",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<ItemBody>>(),
        },
    );
    map.insert(
        "updateItem",
        SchemaBinding {
            request: Some(schema_ref::<ItemPatch>()),
            response: schema_ref::<ItemBody>(),
        },
    );
    map.insert(
        "resolveItemByNumber",
        SchemaBinding {
            request: None,
            response: schema_ref::<ResolveIdBody>(),
        },
    );
    map.insert(
        "createItem",
        SchemaBinding {
            request: Some(schema_ref::<ItemCreate>()),
            response: schema_ref::<ItemBody>(),
        },
    );
    map.insert(
        "releaseItem",
        SchemaBinding {
            request: None,
            response: schema_ref::<ItemBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<ItemBody>(schemas);
    merge_type::<ListBody<ItemBody>>(schemas);
    merge_type::<ItemPatch>(schemas);
    merge_type::<ItemCreate>(schemas);
}
