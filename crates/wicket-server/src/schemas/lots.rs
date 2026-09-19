//! Body schemas for lot and serial operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use serde_json::Value;
use wicket_mod_lots::{LotBody, SerialBody};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getLot",
        SchemaBinding {
            request: None,
            response: schema_ref::<LotBody>(),
        },
    );
    map.insert(
        "listSerials",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<SerialBody>>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LotBody>(schemas);
    merge_type::<SerialBody>(schemas);
    merge_type::<ListBody<SerialBody>>(schemas);
}
