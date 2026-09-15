//! One capability table (ADR 0010 / T-24).
//!
//! Assembled from kernel operations plus the **mounted** subset of first-party
//! `module.toml` routes. Unmounted module routes (item list, inventory issues,
//! …) stay off this table until T-30. Handlers stay hand-written; this module
//! names what is mounted.

/// Kind of capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    /// Ordinary HTTP operation.
    Http,
    /// State-machine edge (`If-Match`, optional signature).
    Transition,
}

/// One row of the capability table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    /// OpenAPI `operationId` and handler bind key.
    pub id: &'static str,
    /// Kind.
    pub kind: CapabilityKind,
    /// HTTP method (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`).
    pub method: &'static str,
    /// Path (`/api/v1/items/{id}`).
    pub path: &'static str,
    /// Manifest permission; empty means unauthenticated.
    pub permission: &'static str,
    /// First-party module id when this row is a mounted `[[routes]]` entry.
    pub module: Option<&'static str>,
    /// State-machine edge name when [`CapabilityKind::Transition`].
    pub edge: Option<&'static str>,
    /// Engine document type for the `(doc_type, edge)` signature join.
    pub doc_type: Option<&'static str>,
}

const fn kernel(
    id: &'static str,
    kind: CapabilityKind,
    method: &'static str,
    path: &'static str,
    permission: &'static str,
    edge: Option<&'static str>,
    doc_type: Option<&'static str>,
) -> Capability {
    Capability {
        id,
        kind,
        method,
        path,
        permission,
        module: None,
        edge,
        doc_type,
    }
}

#[allow(clippy::too_many_arguments)]
const fn module(
    id: &'static str,
    kind: CapabilityKind,
    method: &'static str,
    path: &'static str,
    permission: &'static str,
    module: &'static str,
    edge: Option<&'static str>,
    doc_type: Option<&'static str>,
) -> Capability {
    Capability {
        id,
        kind,
        method,
        path,
        permission,
        module: Some(module),
        edge,
        doc_type,
    }
}

/// Kernel (and kernel-crate) operations. Not declared in `modules/*/module.toml`.
pub const KERNEL: &[Capability] = &[
    kernel(
        "health",
        CapabilityKind::Http,
        "GET",
        "/health",
        "",
        None,
        None,
    ),
    kernel(
        "getOpenApi",
        CapabilityKind::Http,
        "GET",
        "/api/v1/openapi.json",
        "",
        None,
        None,
    ),
    kernel(
        "getValidationManifest",
        CapabilityKind::Http,
        "GET",
        "/api/v1/iq/manifest",
        "validation.manifest.read",
        None,
        None,
    ),
    kernel(
        "exportAudit",
        CapabilityKind::Http,
        "GET",
        "/api/v1/audit",
        "audit.export",
        None,
        None,
    ),
    kernel(
        "getNavigation",
        CapabilityKind::Http,
        "GET",
        "/api/v1/navigation",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "login",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/login",
        "",
        None,
        None,
    ),
    kernel(
        "logout",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/logout",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "createPrincipal",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/principals",
        "identity.manage",
        None,
        None,
    ),
    kernel(
        "getPrincipal",
        CapabilityKind::Http,
        "GET",
        "/api/v1/identity/principals/{id}",
        "identity.manage",
        None,
        None,
    ),
    kernel(
        "renamePrincipal",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/principals/{id}/rename",
        "identity.manage",
        None,
        None,
    ),
    kernel(
        "deactivatePrincipal",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/principals/{id}/deactivate",
        "identity.manage",
        None,
        None,
    ),
    kernel(
        "resetLoginCredential",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/principals/{id}/login-credential",
        "identity.manage",
        None,
        None,
    ),
    kernel(
        "getOwnProfile",
        CapabilityKind::Http,
        "GET",
        "/api/v1/identity/me",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "changeOwnLoginCredential",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/me/login-credential",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "setOwnSigningCredential",
        CapabilityKind::Http,
        "POST",
        "/api/v1/identity/me/signing-credential",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "approveCalibration",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/calibration/certificates/{id}/approve",
        "calibration.approve",
        Some("approve"),
        Some("calibration.certificate"),
    ),
    kernel(
        "esignChallenge",
        CapabilityKind::Http,
        "POST",
        "/api/v1/esign/challenges",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "esignMint",
        CapabilityKind::Http,
        "POST",
        "/api/v1/esign/signatures",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "getEsignSignature",
        CapabilityKind::Http,
        "GET",
        "/api/v1/esign/signatures/{id}",
        "identity.session",
        None,
        None,
    ),
    kernel(
        "getEsignBundle",
        CapabilityKind::Http,
        "GET",
        "/api/v1/esign/signatures/{id}/bundle",
        "esign.bundle.read",
        None,
        None,
    ),
    kernel(
        "defineCustomField",
        CapabilityKind::Http,
        "POST",
        "/api/v1/customfields/definitions",
        "customfields.define",
        None,
        None,
    ),
    kernel(
        "listCustomFieldDefinitions",
        CapabilityKind::Http,
        "GET",
        "/api/v1/customfields/definitions",
        "customfields.view",
        None,
        None,
    ),
    kernel(
        "retireCustomField",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/customfields/definitions/{id}/retire",
        "customfields.retire",
        Some("retire"),
        Some("customfields.definition"),
    ),
    kernel(
        "setItemCustomFields",
        CapabilityKind::Http,
        "PUT",
        "/api/v1/items/{id}/custom-fields",
        "customfields.set",
        None,
        None,
    ),
    kernel(
        "getItemCustomFields",
        CapabilityKind::Http,
        "GET",
        "/api/v1/items/{id}/custom-fields",
        "customfields.view",
        None,
        None,
    ),
    kernel(
        "createDocument",
        CapabilityKind::Http,
        "POST",
        "/api/v1/documents",
        "documents.edit",
        None,
        None,
    ),
    kernel(
        "getDocument",
        CapabilityKind::Http,
        "GET",
        "/api/v1/documents/{id}",
        "documents.view",
        None,
        None,
    ),
    kernel(
        "createDocumentRevision",
        CapabilityKind::Http,
        "POST",
        "/api/v1/documents/{id}/revisions",
        "documents.edit",
        None,
        None,
    ),
    kernel(
        "submitDocument",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/documents/{id}/submit",
        "documents.edit",
        Some("submit"),
        Some("document"),
    ),
    kernel(
        "approveDocument",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/documents/{id}/approve",
        "documents.approve",
        Some("approve"),
        Some("document"),
    ),
    kernel(
        "listPrintTemplates",
        CapabilityKind::Http,
        "GET",
        "/api/v1/print/templates",
        "print.templates",
        None,
        None,
    ),
    kernel(
        "renderPrint",
        CapabilityKind::Http,
        "POST",
        "/api/v1/print/render",
        "print.render",
        None,
        None,
    ),
    kernel(
        "archivePrint",
        CapabilityKind::Http,
        "POST",
        "/api/v1/print/archive",
        "print.archive",
        None,
        None,
    ),
    kernel(
        "releaseFromQuarantine",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/inventory/releases",
        "lots.release",
        Some("release"),
        Some("lot"),
    ),
    kernel(
        "reverseIssue",
        CapabilityKind::Http,
        "POST",
        "/api/v1/inventory/reversals",
        "inventory.adjust",
        None,
        None,
    ),
];

/// Mounted first-party `[[routes]]`. Path+method exists in that module's toml.
pub const MODULE: &[Capability] = &[
    module(
        "listItems",
        CapabilityKind::Http,
        "GET",
        "/api/v1/items",
        "items.view",
        "mod-items",
        None,
        None,
    ),
    module(
        "createItem",
        CapabilityKind::Http,
        "POST",
        "/api/v1/items",
        "items.edit",
        "mod-items",
        None,
        None,
    ),
    module(
        "getItem",
        CapabilityKind::Http,
        "GET",
        "/api/v1/items/{id}",
        "items.view",
        "mod-items",
        None,
        None,
    ),
    module(
        "updateItem",
        CapabilityKind::Http,
        "PATCH",
        "/api/v1/items/{id}",
        "items.edit",
        "mod-items",
        None,
        None,
    ),
    module(
        "releaseItem",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/items/{id}/release",
        "items.release",
        "mod-items",
        Some("release"),
        Some("items"),
    ),
    module(
        "listLocations",
        CapabilityKind::Http,
        "GET",
        "/api/v1/locations",
        "locations.view",
        "mod-locations",
        None,
        None,
    ),
    module(
        "createLocation",
        CapabilityKind::Http,
        "POST",
        "/api/v1/locations",
        "locations.edit",
        "mod-locations",
        None,
        None,
    ),
    module(
        "getLocation",
        CapabilityKind::Http,
        "GET",
        "/api/v1/locations/{id}",
        "locations.view",
        "mod-locations",
        None,
        None,
    ),
    module(
        "listLocationTree",
        CapabilityKind::Http,
        "GET",
        "/api/v1/locations/tree",
        "locations.view",
        "mod-locations",
        None,
        None,
    ),
    module(
        "deactivateLocation",
        CapabilityKind::Http,
        "POST",
        "/api/v1/locations/{id}/deactivate",
        "locations.edit",
        "mod-locations",
        None,
        None,
    ),
    module(
        "listLots",
        CapabilityKind::Http,
        "GET",
        "/api/v1/lots",
        "lots.view",
        "mod-lots",
        None,
        None,
    ),
    module(
        "createLot",
        CapabilityKind::Http,
        "POST",
        "/api/v1/lots",
        "lots.edit",
        "mod-lots",
        None,
        None,
    ),
    module(
        "getLot",
        CapabilityKind::Http,
        "GET",
        "/api/v1/lots/{id}",
        "lots.view",
        "mod-lots",
        None,
        None,
    ),
    module(
        "setLotStatus",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/lots/{id}/status",
        "lots.release",
        "mod-lots",
        Some("release"),
        Some("lot"),
    ),
    module(
        "listPackages",
        CapabilityKind::Http,
        "GET",
        "/api/v1/lots/{id}/packages",
        "lots.view",
        "mod-lots",
        None,
        None,
    ),
    module(
        "createPackage",
        CapabilityKind::Http,
        "POST",
        "/api/v1/lots/{id}/packages",
        "lots.edit",
        "mod-lots",
        None,
        None,
    ),
    module(
        "listSerials",
        CapabilityKind::Http,
        "GET",
        "/api/v1/lots/{id}/serials",
        "lots.view",
        "mod-lots",
        None,
        None,
    ),
    module(
        "createSerials",
        CapabilityKind::Http,
        "POST",
        "/api/v1/lots/{id}/serials",
        "lots.edit",
        "mod-lots",
        None,
        None,
    ),
    module(
        "createReceipt",
        CapabilityKind::Http,
        "POST",
        "/api/v1/inventory/receipts",
        "inventory.receive",
        "mod-inventory",
        None,
        None,
    ),
    module(
        "createCount",
        CapabilityKind::Http,
        "POST",
        "/api/v1/inventory/counts",
        "inventory.count",
        "mod-inventory",
        None,
        None,
    ),
    module(
        "getOnHand",
        CapabilityKind::Http,
        "GET",
        "/api/v1/inventory/on-hand",
        "inventory.view",
        "mod-inventory",
        None,
        None,
    ),
    module(
        "listWorkOrders",
        CapabilityKind::Http,
        "GET",
        "/api/v1/work-orders",
        "production.view",
        "mod-production-min",
        None,
        None,
    ),
    module(
        "createWorkOrder",
        CapabilityKind::Http,
        "POST",
        "/api/v1/work-orders",
        "production.create",
        "mod-production-min",
        None,
        None,
    ),
    module(
        "getWorkOrder",
        CapabilityKind::Http,
        "GET",
        "/api/v1/work-orders/{id}",
        "production.view",
        "mod-production-min",
        None,
        None,
    ),
    module(
        "releaseWorkOrder",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/work-orders/{id}/release",
        "production.release",
        "mod-production-min",
        Some("release"),
        Some("production"),
    ),
    module(
        "issueWorkOrder",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/work-orders/{id}/issue",
        "production.issue",
        "mod-production-min",
        Some("issue"),
        Some("production"),
    ),
    module(
        "completeWorkOrder",
        CapabilityKind::Transition,
        "POST",
        "/api/v1/work-orders/{id}/complete",
        "production.complete",
        "mod-production-min",
        Some("complete"),
        Some("production"),
    ),
    module(
        "traceGenealogy",
        CapabilityKind::Http,
        "GET",
        "/api/v1/genealogy/trace",
        "genealogy.view",
        "mod-genealogy",
        None,
        None,
    ),
    module(
        "getImpact",
        CapabilityKind::Http,
        "GET",
        "/api/v1/genealogy/impact/{lot}",
        "genealogy.view",
        "mod-genealogy",
        None,
        None,
    ),
    module(
        "getGenealogyJob",
        CapabilityKind::Http,
        "GET",
        "/api/v1/genealogy/jobs/{id}",
        "genealogy.view",
        "mod-genealogy",
        None,
        None,
    ),
];

/// The capability table: kernel operations then mounted module routes.
pub fn table() -> impl Iterator<Item = &'static Capability> {
    KERNEL.iter().chain(MODULE.iter())
}

/// Method+path pairs in table order, sorted (T-25 / former `mounted_operations`).
pub fn operations() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = table()
        .map(|c| (c.method.to_string(), c.path.to_string()))
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wicket_module::compiled_in;

    #[test]
    fn ids_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for cap in table() {
            assert!(seen.insert(cap.id), "duplicate capability id {}", cap.id);
        }
    }

    #[test]
    fn method_path_pairs_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for cap in table() {
            assert!(
                seen.insert((cap.method, cap.path)),
                "duplicate {} {}",
                cap.method,
                cap.path
            );
        }
    }

    #[test]
    fn module_rows_exist_in_toml() {
        let catalog = compiled_in().expect("compiled_in");
        for cap in MODULE {
            let module_id = cap.module.expect("MODULE row has module");
            let m = catalog
                .iter()
                .find(|m| m.id == module_id)
                .unwrap_or_else(|| panic!("compiled_in missing {module_id}"));
            assert!(
                m.routes
                    .iter()
                    .any(|r| r.method == cap.method && r.path == cap.path),
                "{module_id} module.toml missing {} {}",
                cap.method,
                cap.path
            );
        }
    }

    #[test]
    fn table_has_the_mounted_count() {
        assert_eq!(
            KERNEL.len() + MODULE.len(),
            65,
            "keep in lockstep with the live mount set"
        );
    }

    #[test]
    fn transition_rows_carry_doc_type_after_edge() {
        let transitions: Vec<_> = table()
            .filter(|c| c.kind == CapabilityKind::Transition)
            .collect();
        assert_eq!(transitions.len(), 10, "Transition count is the mounted set");
        for cap in &transitions {
            assert!(cap.edge.is_some(), "{} missing edge", cap.id);
            assert!(cap.doc_type.is_some(), "{} missing doc_type", cap.id);
        }
        for cap in table().filter(|c| c.kind == CapabilityKind::Http) {
            assert!(cap.edge.is_none(), "{} Http must not carry edge", cap.id);
            assert!(
                cap.doc_type.is_none(),
                "{} Http must not carry doc_type",
                cap.id
            );
        }
        let set_lot = transitions
            .iter()
            .find(|c| c.id == "setLotStatus")
            .expect("setLotStatus");
        assert_eq!(set_lot.edge, Some("release"));
        assert_eq!(set_lot.doc_type, Some("lot"));
    }
}
