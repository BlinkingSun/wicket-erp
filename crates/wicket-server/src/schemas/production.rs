//! Body schemas for work-order and genealogy operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::ResolveIdBody;
use serde_json::{Value, json};
use wicket_jobs::JobStatus;
use wicket_mod_genealogy::{AcceptedBody, Impact, TraceBody};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "traceGenealogy",
        SchemaBinding {
            request: None,
            response: json!({
                "oneOf": [
                    schema_ref::<TraceBody>(),
                    schema_ref::<AcceptedBody>(),
                ]
            }),
        },
    );
    map.insert(
        "getImpact",
        SchemaBinding {
            request: None,
            response: schema_ref::<Impact>(),
        },
    );
    map.insert(
        "getGenealogyJob",
        SchemaBinding {
            request: None,
            response: schema_ref::<JobStatus>(),
        },
    );
    map.insert(
        "resolveWorkOrderByNumber",
        SchemaBinding {
            request: None,
            response: schema_ref::<ResolveIdBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<TraceBody>(schemas);
    merge_type::<AcceptedBody>(schemas);
    merge_type::<Impact>(schemas);
    merge_type::<JobStatus>(schemas);
}
