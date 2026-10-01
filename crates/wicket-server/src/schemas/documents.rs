//! Body schemas for controlled-document operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::documents::{
    DocumentBody, DocumentCreate, DocumentRevisionBody, RevisionCreate,
};
use serde_json::Value;

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "createDocument",
        SchemaBinding {
            request: Some(schema_ref::<DocumentCreate>()),
            response: schema_ref::<DocumentBody>(),
        },
    );
    map.insert(
        "getDocument",
        SchemaBinding {
            request: None,
            response: schema_ref::<DocumentBody>(),
        },
    );
    map.insert(
        "createDocumentRevision",
        SchemaBinding {
            request: Some(schema_ref::<RevisionCreate>()),
            response: schema_ref::<DocumentRevisionBody>(),
        },
    );
    map.insert(
        "submitDocument",
        SchemaBinding {
            request: None,
            response: schema_ref::<DocumentBody>(),
        },
    );
    map.insert(
        "approveDocument",
        SchemaBinding {
            request: None,
            response: schema_ref::<DocumentBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<DocumentBody>(schemas);
    merge_type::<DocumentCreate>(schemas);
    merge_type::<DocumentRevisionBody>(schemas);
    merge_type::<RevisionCreate>(schemas);
}
