//! Body schemas for identity operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::identity::{PrincipalBody, RoleBody};
use serde_json::Value;

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getOwnProfile",
        SchemaBinding {
            request: None,
            response: schema_ref::<PrincipalBody>(),
        },
    );
    map.insert(
        "listPrincipals",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<PrincipalBody>>(),
        },
    );
    map.insert(
        "getPrincipalByUsername",
        SchemaBinding {
            request: None,
            response: schema_ref::<PrincipalBody>(),
        },
    );
    map.insert(
        "listRoles",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<RoleBody>>(),
        },
    );
    map.insert(
        "getRoleByName",
        SchemaBinding {
            request: None,
            response: schema_ref::<RoleBody>(),
        },
    );
    map.insert(
        "listRolesForPrincipal",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<RoleBody>>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<PrincipalBody>(schemas);
    merge_type::<ListBody<PrincipalBody>>(schemas);
    merge_type::<RoleBody>(schemas);
    merge_type::<ListBody<RoleBody>>(schemas);
}
