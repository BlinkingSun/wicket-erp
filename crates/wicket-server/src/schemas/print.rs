//! Body schemas for print operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::print::{
    ArchiveBody, ArchivePrintResponse, RecordBody, RenderBody, RenderPrintResponse,
};
use serde_json::Value;
use wicket_print::TemplateSummary;

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "listPrintTemplates",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<TemplateSummary>>(),
        },
    );
    map.insert(
        "renderPrint",
        SchemaBinding {
            request: Some(schema_ref::<RenderBody>()),
            response: schema_ref::<RenderPrintResponse>(),
        },
    );
    map.insert(
        "archivePrint",
        SchemaBinding {
            request: Some(schema_ref::<ArchiveBody>()),
            response: schema_ref::<ArchivePrintResponse>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<TemplateSummary>(schemas);
    merge_type::<ListBody<TemplateSummary>>(schemas);
    merge_type::<RecordBody>(schemas);
    merge_type::<RenderBody>(schemas);
    merge_type::<ArchiveBody>(schemas);
    merge_type::<RenderPrintResponse>(schemas);
    merge_type::<ArchivePrintResponse>(schemas);
}
