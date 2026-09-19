//! Body schemas for kernel operations (login, navigation).

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::{LoginBody, LoginResponse, NavigationBody};
use serde_json::Value;

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
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LoginBody>(schemas);
    merge_type::<LoginResponse>(schemas);
    merge_type::<NavigationBody>(schemas);
}
