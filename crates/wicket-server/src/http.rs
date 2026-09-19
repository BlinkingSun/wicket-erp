//! Router generated from [`crate::capabilities`] (ADR 0010 / T-24).
//!
//! Handlers stay hand-written. A string-literal route path here is fail-class
//! (`scripts/lint-mounts.sh`).

use std::collections::BTreeMap;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{MethodRouter, get, patch, post, put};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::boot::AppState;
use crate::capabilities::{self, Capability};
use crate::envelope::{RequestId, error_response};
use crate::error::{Error, Result};
use crate::extract::{limits_mw, request_id_mw};
use crate::handlers;

/// Bind one capability id to its handler. Unknown ids are fail-class.
fn method_router(cap: &Capability) -> MethodRouter<AppState> {
    match cap.id {
        "health" => get(handlers::health),
        "getOpenApi" => get(handlers::openapi),
        "getValidationManifest" => get(handlers::manifest),
        "exportAudit" => get(handlers::audit_export),
        "getNavigation" => get(handlers::navigation),
        "login" => post(handlers::login),
        "logout" => post(handlers::logout),
        "createPrincipal" => post(handlers::identity::create_principal),
        "listPrincipals" => get(handlers::identity::list_principals),
        "getPrincipal" => get(handlers::identity::get_principal),
        "getPrincipalByUsername" => get(handlers::identity::get_principal_by_username),
        "listRoles" => get(handlers::identity::list_roles),
        "getRoleByName" => get(handlers::identity::get_role_by_name),
        "listRolesForPrincipal" => get(handlers::identity::list_roles_for_principal),
        "renamePrincipal" => post(handlers::identity::rename_principal),
        "deactivatePrincipal" => post(handlers::identity::deactivate_principal),
        "resetLoginCredential" => post(handlers::identity::reset_login_credential),
        "getOwnProfile" => get(handlers::identity::get_own_profile),
        "changeOwnLoginCredential" => post(handlers::identity::change_own_login_credential),
        "setOwnSigningCredential" => post(handlers::identity::set_own_signing_credential),
        "approveCalibration" => post(handlers::approve_calibration),
        "esignChallenge" => post(handlers::esign_challenge),
        "esignMint" => post(handlers::esign_mint),
        "getEsignSignature" => get(handlers::esign_manifestation),
        "getEsignBundle" => get(handlers::esign_bundle),
        "defineCustomField" => post(handlers::customfields::define),
        "listCustomFieldDefinitions" => get(handlers::customfields::definitions_for),
        "retireCustomField" => post(handlers::customfields::retire),
        "setItemCustomFields" => put(handlers::customfields::set),
        "getItemCustomFields" => get(handlers::customfields::get_item_fields),
        "createDocument" => post(handlers::documents::create_document),
        "getDocument" => get(handlers::documents::get_document),
        "createDocumentRevision" => post(handlers::documents::create_revision),
        "submitDocument" => post(handlers::documents::submit_document),
        "approveDocument" => post(handlers::documents::approve_document),
        "listPrintTemplates" => get(handlers::print::list_templates),
        "renderPrint" => post(handlers::print::render),
        "archivePrint" => post(handlers::print::archive),
        "releaseFromQuarantine" => post(handlers::release_stock),
        "reverseIssue" => post(handlers::create_reversal),
        "listItems" => get(handlers::list_items),
        "createItem" => post(handlers::create_item),
        "getItem" => get(handlers::get_item),
        "resolveItemByNumber" => get(handlers::resolve_item),
        "updateItem" => patch(handlers::update_item),
        "releaseItem" => post(handlers::release_item),
        "listLocations" => get(handlers::list_locations),
        "createLocation" => post(handlers::create_location),
        "getLocation" => get(handlers::get_location),
        "listLocationTree" => get(handlers::list_location_tree),
        "deactivateLocation" => post(handlers::deactivate_location),
        "listLots" => get(handlers::list_lots),
        "createLot" => post(handlers::create_lot),
        "getLot" => get(handlers::get_lot),
        "setLotStatus" => post(handlers::set_lot_status),
        "listPackages" => get(handlers::list_packages),
        "createPackage" => post(handlers::create_package),
        "listSerials" => get(handlers::list_serials),
        "createSerials" => post(handlers::create_serials),
        "createReceipt" => post(handlers::create_receipt),
        "createCount" => post(handlers::create_count),
        "getOnHand" => get(handlers::on_hand),
        "listWorkOrders" => get(handlers::list_work_orders),
        "createWorkOrder" => post(handlers::create_wo),
        "getWorkOrder" => get(handlers::get_wo),
        "resolveWorkOrderByNumber" => get(handlers::resolve_work_order),
        "releaseWorkOrder" => post(handlers::release_wo),
        "issueWorkOrder" => post(handlers::issue_wo),
        "completeWorkOrder" => post(handlers::complete_wo),
        "traceGenealogy" => get(handlers::genealogy_trace),
        "getImpact" => get(handlers::get_impact),
        "getGenealogyJob" => get(handlers::get_genealogy_job),
        id => panic!("T-24: no handler bound for capability {id}"),
    }
}

/// Build the HTTP router from the capability table. Does not bind a port.
pub fn router(state: AppState) -> Router {
    let mut by_path: BTreeMap<&'static str, MethodRouter<AppState>> = BTreeMap::new();
    for cap in capabilities::table() {
        let next = method_router(cap);
        match by_path.remove(cap.path) {
            Some(prev) => {
                by_path.insert(cap.path, prev.merge(next));
            }
            None => {
                by_path.insert(cap.path, next);
            }
        }
    }
    let mut r = Router::<AppState>::new();
    for (path, mr) in by_path {
        r = r.route(path, mr);
    }
    // Capability table is exhaustive for /api. Fallback never shadows it:
    // unmatched /api/** stays the JSON error envelope, never index.html.
    r.fallback(static_or_api_404)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(limits_mw))
        .layer(axum::middleware::from_fn(request_id_mw))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

fn is_api_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

async fn static_or_api_404(
    State(state): State<AppState>,
    req: Request,
) -> axum::response::Response {
    if is_api_path(req.uri().path()) {
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .map(|r| r.as_str())
            .unwrap_or("00000000-0000-0000-0000-000000000000");
        return error_response(Error::not_found("not found"), request_id);
    }
    let Some(root) = state.ui_root() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let index = root.join("index.html");
    let svc = ServeDir::new(root).fallback(ServeFile::new(index));
    match svc.oneshot(req).await {
        Ok(resp) => resp.into_response(),
        Err(infallible) => match infallible {},
    }
}

/// Bind and serve.
pub async fn serve(state: AppState) -> Result<()> {
    let bind: SocketAddr = state.bind();
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| crate::error::Error::Config(format!("bind {bind}: {e}")))?;
    tracing::info!(%bind, "wicket listening");
    axum::serve(listener, router(state))
        .await
        .map_err(|e| crate::error::Error::Config(format!("serve: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::method_router;
    use crate::capabilities;

    #[test]
    fn every_capability_has_a_handler() {
        for cap in capabilities::table() {
            let _ = method_router(cap);
        }
    }
}
