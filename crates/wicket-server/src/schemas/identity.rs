//! Body schemas for identity operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::identity::{
    CreatePrincipalBody, EmptyBody, NoContentJson, PasswordBody, PrincipalBody, RenameBody,
    RoleBody, SigningSecretBody,
};
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
    map.insert(
        "createPrincipal",
        SchemaBinding {
            request: Some(schema_ref::<CreatePrincipalBody>()),
            response: schema_ref::<PrincipalBody>(),
        },
    );
    map.insert(
        "getPrincipal",
        SchemaBinding {
            request: None,
            response: schema_ref::<PrincipalBody>(),
        },
    );
    map.insert(
        "renamePrincipal",
        SchemaBinding {
            request: Some(schema_ref::<RenameBody>()),
            response: schema_ref::<NoContentJson>(),
        },
    );
    map.insert(
        "deactivatePrincipal",
        SchemaBinding {
            request: Some(schema_ref::<EmptyBody>()),
            response: schema_ref::<NoContentJson>(),
        },
    );
    map.insert(
        "resetLoginCredential",
        SchemaBinding {
            request: Some(schema_ref::<PasswordBody>()),
            response: schema_ref::<NoContentJson>(),
        },
    );
    map.insert(
        "changeOwnLoginCredential",
        SchemaBinding {
            request: Some(schema_ref::<PasswordBody>()),
            response: schema_ref::<NoContentJson>(),
        },
    );
    map.insert(
        "setOwnSigningCredential",
        SchemaBinding {
            request: Some(schema_ref::<SigningSecretBody>()),
            response: schema_ref::<NoContentJson>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<PrincipalBody>(schemas);
    merge_type::<ListBody<PrincipalBody>>(schemas);
    merge_type::<RoleBody>(schemas);
    merge_type::<ListBody<RoleBody>>(schemas);
    merge_type::<CreatePrincipalBody>(schemas);
    merge_type::<RenameBody>(schemas);
    merge_type::<PasswordBody>(schemas);
    merge_type::<SigningSecretBody>(schemas);
    merge_type::<EmptyBody>(schemas);
    merge_type::<NoContentJson>(schemas);
}
