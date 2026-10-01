//! Body schemas for inventory operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::inventory::{
    CountBody, ReceiptBody, ReleaseInvBody, ReversalBody, ReverseIssueBody,
};
use crate::handlers::{OnHandBody, ReceiptLine};
use serde_json::Value;
use wicket_mod_inventory::{DocumentBody, LineBody, QuantityBody};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getOnHand",
        SchemaBinding {
            request: None,
            response: schema_ref::<OnHandBody>(),
        },
    );
    map.insert(
        "releaseFromQuarantine",
        SchemaBinding {
            request: Some(schema_ref::<ReleaseInvBody>()),
            response: schema_ref::<DocumentBody>(),
        },
    );
    map.insert(
        "reverseIssue",
        SchemaBinding {
            request: Some(schema_ref::<ReversalBody>()),
            response: schema_ref::<ReverseIssueBody>(),
        },
    );
    map.insert(
        "createReceipt",
        SchemaBinding {
            request: Some(schema_ref::<ReceiptBody>()),
            response: schema_ref::<DocumentBody>(),
        },
    );
    map.insert(
        "createCount",
        SchemaBinding {
            request: Some(schema_ref::<CountBody>()),
            response: schema_ref::<DocumentBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<OnHandBody>(schemas);
    merge_type::<DocumentBody>(schemas);
    merge_type::<LineBody>(schemas);
    merge_type::<QuantityBody>(schemas);
    merge_type::<ReverseIssueBody>(schemas);
    merge_type::<ReleaseInvBody>(schemas);
    merge_type::<ReversalBody>(schemas);
    merge_type::<ReceiptBody>(schemas);
    merge_type::<ReceiptLine>(schemas);
    merge_type::<CountBody>(schemas);
}
