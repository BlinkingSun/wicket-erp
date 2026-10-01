//! Location HTTP handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wicket_core::LocationId;
use wicket_db::Tx;
use wicket_mod_locations::{CreateLocation, Location, LocationKind};

use super::{H, ListQ, json_status, nonempty, parse_json, parse_limit, rid};
use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract::{self, require_if_match};
use crate::idempotency;
use crate::session::write_context;
use crate::wire::parse_uuid;

use axum::body::Bytes;

fn map_locations_err(e: wicket_mod_locations::Error) -> Error {
    match e {
        wicket_mod_locations::Error::Validation(ref msg) => {
            let field = if msg == "limit" {
                Some("limit")
            } else if msg == "cursor" {
                Some("cursor")
            } else {
                None
            };
            Error::validation(e.to_string(), field)
        }
        wicket_mod_locations::Error::OnHand | wicket_mod_locations::Error::Protected => {
            Error::http("REFUSED", e.to_string(), None, StatusCode::CONFLICT)
        }
        wicket_mod_locations::Error::Immutable(_) | wicket_mod_locations::Error::Cycle => {
            Error::validation(e.to_string(), None)
        }
        other => other.into(),
    }
}

pub async fn create_location(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match create_loc_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

/// POST `/api/v1/locations` body.
#[derive(Deserialize, JsonSchema)]
pub struct LocCreate {
    code: String,
    name: String,
    #[serde(default)]
    kind: Option<String>,
}

/// Wire body for `createLocation` and `getLocation`.
///
/// Five fields only. `listLocations` and `deactivateLocation` serialize the
/// full [`Location`] row instead.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LocationSummary {
    id: LocationId,
    code: String,
    name: String,
    kind: LocationKind,
    version: i64,
}

impl From<&Location> for LocationSummary {
    fn from(loc: &Location) -> Self {
        Self {
            id: loc.id,
            code: loc.code.clone(),
            name: loc.name.clone(),
            kind: loc.kind,
            version: loc.version,
        }
    }
}

async fn create_loc_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: LocCreate = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "locations.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let kind = match body.kind.as_deref().unwrap_or("warehouse") {
        "warehouse" => LocationKind::Warehouse,
        "area" => LocationKind::Area,
        "bin" => LocationKind::Bin,
        "wip" => LocationKind::Wip,
        other => {
            return Err(Error::validation(
                format!("unknown kind {other}"),
                Some("kind"),
            ));
        }
    };
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "locations.create",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let site = wicket_mod_locations::default_site_id(&mut tx).await?;
    let loc = wicket_mod_locations::create(
        &mut tx,
        CreateLocation {
            code: body.code,
            name: body.name,
            site_id: site,
            parent_id: None,
            kind,
        },
    )
    .await?;
    let body = serde_json::to_value(LocationSummary::from(&loc))?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// GET /api/v1/locations/{id}
pub async fn get_location(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_location_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_location_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "locations.view").await?;
    let loc_id = parse_uuid(id, "id", LocationId::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "locations.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let loc = wicket_mod_locations::get(&mut tx, loc_id).await;
    tx.rollback().await?;
    let loc = loc?;
    Ok(serde_json::to_value(LocationSummary::from(&loc))?)
}

pub async fn list_locations(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<ListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_locations_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_locations_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: ListQ,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "locations.view").await?;
    let limit = parse_limit(nonempty(&q.limit))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "locations.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_mod_locations::list_locations(&mut tx, limit, nonempty(&q.cursor)).await;
    tx.rollback().await?;
    serde_json::to_value(&page.map_err(map_locations_err)?).map_err(Error::from)
}

#[derive(Debug, Default, Deserialize)]
pub struct TreeQ {
    #[serde(default)]
    include_inactive: Option<String>,
}

fn parse_include_inactive(raw: Option<&str>) -> Result<bool> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(false),
        Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        Some(other) => Err(Error::validation(
            format!("include_inactive must be true or false, got {other}"),
            Some("include_inactive"),
        )),
    }
}

/// GET /api/v1/locations/tree
pub async fn list_location_tree(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<TreeQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_location_tree_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_location_tree_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: TreeQ,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "locations.view").await?;
    let include_inactive = parse_include_inactive(nonempty(&q.include_inactive))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "locations.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_mod_locations::list_tree(&mut tx, include_inactive).await;
    tx.rollback().await?;
    serde_json::to_value(&page.map_err(map_locations_err)?).map_err(Error::from)
}

/// POST /api/v1/locations/{id}/deactivate
pub async fn deactivate_location(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match deactivate_location_inner(&state, &headers, &request_id, &id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn deactivate_location_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let session = extract::require_mutation(state, headers, request_id, "locations.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let loc_id = parse_uuid(id, "id", LocationId::from_uuid)?;
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "locations.deactivate",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let loc = wicket_mod_locations::deactivate_location(&mut tx, loc_id, expected)
        .await
        .map_err(map_locations_err)?;
    let body = serde_json::to_value(&loc)?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wicket_core::Identifier;
    use wicket_mod_locations::LocationStatus;

    fn sample_location() -> Location {
        Location {
            id: LocationId::from_uuid(uuid::Uuid::nil()),
            code: "WH-1".into(),
            name: "Main".into(),
            site_id: Identifier::from_uuid(uuid::Uuid::nil()),
            parent_id: None,
            kind: LocationKind::Warehouse,
            boundary_class: None,
            status: LocationStatus::Active,
            work_order_id: None,
            version: 1,
        }
    }

    #[test]
    fn location_summary_matches_former_json_blob() {
        let loc = sample_location();
        let blob = json!({
            "id": loc.id.to_string(),
            "code": loc.code,
            "name": loc.name,
            "kind": loc.kind.as_sql(),
            "version": loc.version,
        });
        let typed = serde_json::to_value(LocationSummary::from(&loc)).expect("summary json");
        assert_eq!(blob, typed);
    }

    #[test]
    fn create_get_blob_is_not_full_location() {
        let loc = sample_location();
        let full = serde_json::to_value(&loc).expect("location json");
        let summary = serde_json::to_value(LocationSummary::from(&loc)).expect("summary json");
        assert_ne!(full, summary);
        assert!(full.get("site_id").is_some());
        assert!(full.get("status").is_some());
        assert!(summary.get("site_id").is_none());
        assert!(summary.get("status").is_none());
    }
}
