//! Lot, package, and serial HTTP handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use wicket_core::{Identifier, LotId};
use wicket_db::Tx;
use wicket_mod_lots::{CreateLotBody, CreateSerialsBody, LotStatus, PackageLevel, SetStatusBody};

use super::{
    H, IntoActor, ListQ, bind_esign_header, blocking, fresh_write, json_status, nonempty,
    parse_json, parse_limit, required_edge_token, rid,
};
use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract::{self, check_version, require_if_match};
use crate::idempotency;
use crate::session::write_context;
use crate::wire::{QuantityBody, parse_uuid};

use axum::body::Bytes;

fn map_lots_err(e: wicket_mod_lots::Error) -> Error {
    match e {
        wicket_mod_lots::Error::InvalidLimit => {
            Error::validation("limit must be between 1 and 200", Some("limit"))
        }
        other => other.into(),
    }
}

/// POST /api/v1/lots
pub async fn create_lot(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match create_lot_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_lot_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: CreateLotBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "lots.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "lots.create",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let lot = wicket_mod_lots::create_lot_http(&mut tx, state.kernel(), &ctx, body).await?;
    let body = serde_json::to_value(&lot)?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// GET /api/v1/lots/{id}
pub async fn get_lot(
    State(state): State<AppState>,
    headers: H,

    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_lot_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_lot_inner(state: &AppState, headers: &H, request_id: &str, id: &str) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "lots.view").await?;
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "lots.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let lot = wicket_mod_lots::get_lot(&mut tx, lot_id).await;
    tx.rollback().await?;
    serde_json::to_value(&lot?).map_err(Error::from)
}

/// POST /api/v1/lots/{id}/status
pub async fn set_lot_status(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    let state2 = state.clone();
    let headers2 = headers.clone();
    let rid2 = request_id.clone();
    let raw = body.to_vec();
    match blocking(&request_id, move || async move {
        set_status_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn set_status_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: SetStatusBody = parse_json(raw)?;
    let perm = match body.status {
        LotStatus::Available => "lots.release",
        _ => "lots.edit",
    };
    let session = extract::require_mutation(state, headers, request_id, perm).await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let edge = match body.status {
        LotStatus::Available => "release",
        LotStatus::Hold => "hold",
        LotStatus::Rejected => "reject",
        LotStatus::Quarantine => {
            return Err(Error::validation(
                "cannot set quarantine via status",
                Some("status"),
            ));
        }
    };
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_lots::DOC_TYPE.into(),
        doc_id: Identifier::from_uuid(lot_id.as_uuid()),
    };
    let mut ctx = state
        .kernel()
        .transition_context(session.principal.0.into_actor(), &doc, edge);
    ctx.session_id = Some(session.id.to_string());
    ctx.request_id = Some(request_id.to_string());
    ctx.source_kind = "api".into();
    ctx.reason = Some("api".into());
    ctx.actor_display = Some(session.display_name.clone());
    bind_esign_header(&mut ctx, headers);
    let write = fresh_write(state).await?;
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let current = wicket_mod_lots::get_lot(&mut tx, lot_id).await?;
    check_version(current.version, expected)?;
    if edge == "release" {
        let _ = required_edge_token(
            state,
            &mut tx,
            headers,
            session.principal.0.into_actor(),
            "Lot released",
            Identifier::from_uuid(lot_id.as_uuid()),
            current.version,
        )
        .await?;
    }
    let lot = wicket_mod_lots::set_status_http(&mut tx, state.kernel(), &ctx, lot_id, body).await?;
    let body = serde_json::to_value(&lot)?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

#[derive(Deserialize)]
pub struct PackageCreate {
    level: String,
    contained: QuantityBody,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    label_ref: Option<String>,
}

/// POST /api/v1/lots/{id}/packages
pub async fn create_package(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match create_pkg_inner(&state, &headers, &request_id, &id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_pkg_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: PackageCreate = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "lots.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let level = PackageLevel::parse(&body.level)
        .map_err(|e| Error::validation(e.to_string(), Some("level")))?;
    let contained = body.contained.to_qty()?;
    let parent = match body.parent_id {
        Some(p) => Some(parse_uuid(&p, "parent_id", |u| {
            wicket_mod_lots::PackageId::from_uuid(u)
        })?),
        None => None,
    };
    let ctx = write_context(
        &session,
        "lots.edit",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let pkg = wicket_mod_lots::create_package(
        &mut tx,
        lot_id,
        parent,
        level,
        contained,
        body.label_ref.as_deref(),
    )
    .await?;
    let body = json!({
        "id": pkg.id.as_uuid().to_string(),
        "lot_id": pkg.lot.to_string(),
        "parent_id": pkg.parent.map(|p| p.as_uuid().to_string()),
        "level": pkg.level.as_str(),
        "contained": QuantityBody::from_qty(&pkg.contained),
    });
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// GET /api/v1/lots/{id}/packages
pub async fn list_packages(
    State(state): State<AppState>,
    headers: H,

    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match list_pkg_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_pkg_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "lots.view").await?;
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "lots.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let tree = wicket_mod_lots::package_hierarchy(&mut tx, lot_id).await;
    tx.rollback().await?;
    let tree = tree?;
    let data: Vec<Value> = tree
        .iter()
        .map(|p| {
            json!({
                "id": p.id.as_uuid().to_string(),
                "parent_id": p.parent.map(|x| x.as_uuid().to_string()),
                "level": p.level.as_str(),
                "contained": QuantityBody::from_qty(&p.contained),
            })
        })
        .collect();
    Ok(json!({"data": data, "next_cursor": null, "has_more": false}))
}

/// GET /api/v1/lots/{id}/serials
pub async fn list_serials(
    State(state): State<AppState>,
    headers: H,

    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match list_serials_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_serials_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "lots.view").await?;
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "lots.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_mod_lots::list_serials(&mut tx, lot_id, Some(200), None).await;
    tx.rollback().await?;
    serde_json::to_value(&page?).map_err(Error::from)
}

/// GET /api/v1/lots
pub async fn list_lots(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<ListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_lots_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_lots_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: ListQ,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "lots.view").await?;
    let limit = parse_limit(nonempty(&q.limit))?.map(i64::from);
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "lots.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_mod_lots::list_lots(&mut tx, limit, nonempty(&q.cursor)).await;
    tx.rollback().await?;
    serde_json::to_value(&page.map_err(map_lots_err)?).map_err(Error::from)
}

/// POST /api/v1/lots/{id}/serials
pub async fn create_serials(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match create_serials_inner(&state, &headers, &request_id, &id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_serials_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: CreateSerialsBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "lots.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let lot_id = parse_uuid(id, "id", LotId::from_uuid)?;
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "lots.edit",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let page = wicket_mod_lots::create_serials_http(&mut tx, state.kernel(), lot_id, body)
        .await
        .map_err(map_lots_err)?;
    let body = serde_json::to_value(&page)?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}
