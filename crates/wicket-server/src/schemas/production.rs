//! Body schemas for work-order and genealogy operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::ResolveIdBody;
use crate::handlers::production::{
    CompleteBody, IssueBody, WoCreate, WorkOrderCompleteJson, WorkOrderJson,
};
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
    map.insert(
        "listWorkOrders",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<WorkOrderJson>>(),
        },
    );
    map.insert(
        "createWorkOrder",
        SchemaBinding {
            request: Some(schema_ref::<WoCreate>()),
            response: schema_ref::<WorkOrderJson>(),
        },
    );
    map.insert(
        "getWorkOrder",
        SchemaBinding {
            request: None,
            response: schema_ref::<WorkOrderJson>(),
        },
    );
    map.insert(
        "releaseWorkOrder",
        SchemaBinding {
            request: None,
            response: schema_ref::<WorkOrderJson>(),
        },
    );
    map.insert(
        "issueWorkOrder",
        SchemaBinding {
            request: Some(schema_ref::<IssueBody>()),
            response: schema_ref::<WorkOrderJson>(),
        },
    );
    map.insert(
        "completeWorkOrder",
        SchemaBinding {
            request: Some(schema_ref::<CompleteBody>()),
            response: schema_ref::<WorkOrderCompleteJson>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<TraceBody>(schemas);
    merge_type::<AcceptedBody>(schemas);
    merge_type::<Impact>(schemas);
    merge_type::<JobStatus>(schemas);
    merge_type::<WorkOrderJson>(schemas);
    merge_type::<ListBody<WorkOrderJson>>(schemas);
    merge_type::<WorkOrderCompleteJson>(schemas);
    merge_type::<WoCreate>(schemas);
    merge_type::<IssueBody>(schemas);
    merge_type::<CompleteBody>(schemas);
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;

    #[test]
    fn binds_the_six_work_order_operations() {
        let mut map = SchemaMap::new();
        register(&mut map);
        assert!(map["listWorkOrders"].request.is_none());
        assert!(map["createWorkOrder"].request.is_some());
        assert!(map["getWorkOrder"].request.is_none());
        assert!(map["releaseWorkOrder"].request.is_none());
        assert!(map["issueWorkOrder"].request.is_some());
        assert!(map["completeWorkOrder"].request.is_some());
        assert_eq!(
            map["listWorkOrders"].response["$ref"],
            format!(
                "#/components/schemas/{}",
                ListBody::<WorkOrderJson>::schema_name()
            )
        );
        assert_eq!(
            map["getWorkOrder"].response["$ref"],
            format!("#/components/schemas/{}", WorkOrderJson::schema_name())
        );
        assert_eq!(
            map["completeWorkOrder"].response["$ref"],
            format!(
                "#/components/schemas/{}",
                WorkOrderCompleteJson::schema_name()
            )
        );
        assert_eq!(WorkOrderJson::schema_name(), "WorkOrderJson");
        assert_eq!(
            ListBody::<WorkOrderJson>::schema_name(),
            "ListBody_for_WorkOrderJson"
        );
        assert_eq!(
            WorkOrderCompleteJson::schema_name(),
            "WorkOrderCompleteJson"
        );
        assert_eq!(WoCreate::schema_name(), "WoCreate");
        assert_eq!(IssueBody::schema_name(), "IssueBody");
        assert_eq!(CompleteBody::schema_name(), "CompleteBody");
    }

    #[test]
    fn merge_inserts_wire_types_and_quantity_refs_any_quantity() {
        let mut schemas = serde_json::Map::new();
        merge_components(&mut schemas);
        for name in [
            "WorkOrderJson",
            "ListBody_for_WorkOrderJson",
            "WorkOrderCompleteJson",
            "FinishedLotRef",
            "WoCreate",
            "IssueBody",
            "CompleteBody",
            "WorkOrderIssueLine",
        ] {
            assert!(
                schemas.contains_key(name),
                "missing {name}; have {:?}",
                schemas.keys().collect::<Vec<_>>()
            );
        }
        let qty = &schemas["WorkOrderJson"]["properties"]["quantity"];
        let r = qty["$ref"].as_str().unwrap_or("");
        assert!(
            r.ends_with("/AnyQuantity"),
            "quantity must use the hand-written AnyQuantity schema: {qty}"
        );
        let complete_qty = &schemas["WorkOrderCompleteJson"]["properties"]["quantity"];
        let cr = complete_qty["$ref"].as_str().unwrap_or("");
        assert!(cr.ends_with("/AnyQuantity"), "{complete_qty}");
    }
}
