//! Body schemas for kernel operations (login, navigation, OpenAPI, manifest, audit).
//!
//! `health` returns `text/plain` and `logout` returns 204 with no body. The
//! registration seam only attaches `application/json` on HTTP 200, so those
//! two ids are not registered (a JSON 200 schema would be a false contract).

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::kernel::{
    AuditExportBody, AuditHeadBody, ManifestModuleBody, OpenApiDocument, SignatureEdgeBody,
    ValidationManifestBody,
};
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
    map.insert(
        "getOpenApi",
        SchemaBinding {
            request: None,
            response: schema_ref::<OpenApiDocument>(),
        },
    );
    map.insert(
        "getValidationManifest",
        SchemaBinding {
            request: None,
            response: schema_ref::<ValidationManifestBody>(),
        },
    );
    map.insert(
        "exportAudit",
        SchemaBinding {
            request: None,
            response: schema_ref::<AuditExportBody>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LoginBody>(schemas);
    merge_type::<LoginResponse>(schemas);
    merge_type::<NavigationBody>(schemas);
    merge_type::<OpenApiDocument>(schemas);
    merge_type::<ValidationManifestBody>(schemas);
    merge_type::<ManifestModuleBody>(schemas);
    merge_type::<SignatureEdgeBody>(schemas);
    merge_type::<AuditExportBody>(schemas);
    merge_type::<AuditHeadBody>(schemas);
}
