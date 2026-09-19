//! Item HTTP handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use wicket_core::{Identifier, ItemId};
use wicket_db::Tx;
use wicket_ledger::CostMethod;
use wicket_mod_items::{Kind, NewItem, UpdateItem};

use super::{
    H, IntoActor, ResolveIdBody, ResolveNumberQ, blocking, fresh_write, json_status, nonempty,
    parse_json, parse_limit, rid,
};
use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract::{self, check_version, require_if_match};
use crate::idempotency;
use crate::session::write_context;
use crate::wire::{MoneyBody, parse_uuid};

use axum::body::Bytes;

fn map_items_err(e: wicket_mod_items::Error) -> Error {
    match e {
        wicket_mod_items::Error::InvalidLimit => {
            Error::validation("limit must be between 1 and 200", Some("limit"))
        }
        wicket_mod_items::Error::StockMeasureImmutable
        | wicket_mod_items::Error::StandardCostRequired
        | wicket_mod_items::Error::InvalidTransition { .. }
        | wicket_mod_items::Error::Manifest(_) => Error::validation(e.to_string(), None),
        other => other.into(),
    }
}

fn parse_cost_method(raw: &str) -> Result<CostMethod> {
    match raw {
        "FIFO" | "Fifo" | "fifo" => Ok(CostMethod::Fifo),
        "MOVING_AVG" | "MovingAvg" => Ok(CostMethod::MovingAvg),
        "STANDARD" | "Standard" => Ok(CostMethod::Standard),
        other => Err(Error::validation(
            format!("unknown cost_method {other}"),
            Some("cost_method"),
        )),
    }
}

#[derive(Deserialize)]
pub struct ItemCreate {
    number: String,
    revision: String,
    description: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
    stock_uom: i64,
    #[serde(default)]
    stock_scale: i16,
    #[serde(default)]
    residual_tolerance: Option<String>,
    #[serde(default)]
    cost_method: Option<String>,
    #[serde(default)]
    standard: Option<MoneyBody>,
    #[serde(default)]
    id: Option<String>,
}

/// POST /api/v1/items
pub async fn create_item(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match create_item_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_item_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ItemCreate = parse_json(raw)?;
    if body.id.is_some() {
        return Err(Error::validation(
            "clients must not mint identifiers",
            Some("id"),
        ));
    }
    let session = extract::require_mutation(state, headers, request_id, "items.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let kind_s = body.kind.or(body.r#type).unwrap_or_else(|| "make".into());
    let kind = Kind::parse(&kind_s).map_err(|e| Error::validation(e.to_string(), Some("kind")))?;
    let residual = body
        .residual_tolerance
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| Error::validation("residual_tolerance", Some("residual_tolerance")))?;
    let method = match body.cost_method.as_deref().unwrap_or("FIFO") {
        "FIFO" | "Fifo" | "fifo" => CostMethod::Fifo,
        "MOVING_AVG" | "MovingAvg" => CostMethod::MovingAvg,
        "STANDARD" | "Standard" => CostMethod::Standard,
        other => {
            return Err(Error::validation(
                format!("unknown cost_method {other}"),
                Some("cost_method"),
            ));
        }
    };
    let new = NewItem {
        number: body.number,
        revision: body.revision,
        description: body.description,
        kind,
        stock_uom: wicket_core::UnitId(body.stock_uom),
        stock_scale: body.stock_scale,
        residual_tolerance: residual,
        cost_method: method,
        standard: match body.standard {
            Some(m) => Some(m.to_money()?),
            None => None,
        },
    };
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "items.create",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let item = wicket_mod_items::create(&mut tx, state.kernel(), new).await?;
    let body = serde_json::to_value(wicket_mod_items::api::ItemBody::from(&item))?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// GET /api/v1/items/{id}
pub async fn get_item(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    if let Err(e) = extract::require_permission(&state, &headers, &request_id, "items.view").await {
        return error_response(e, &request_id);
    }
    let item_id = match parse_uuid(&id, "id", ItemId::from_uuid) {
        Ok(i) => i,
        Err(e) => return error_response(e, &request_id),
    };
    match wicket_mod_items::get(state.pool(), item_id).await {
        Ok(item) => Json(wicket_mod_items::api::ItemBody::from(&item)).into_response(),
        Err(e) => error_response(e.into(), &request_id),
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ItemListQ {
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    number_prefix: Option<String>,
}

/// GET /api/v1/items
pub async fn list_items(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<ItemListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_items_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_items_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: ItemListQ,
) -> Result<Value> {
    let _session = extract::require_permission(state, headers, request_id, "items.view").await?;
    let limit = parse_limit(nonempty(&q.limit))?;
    let cursor = match nonempty(&q.cursor) {
        Some(c) => Some(parse_uuid(c, "cursor", ItemId::from_uuid)?),
        None => None,
    };
    let kind = match nonempty(&q.kind) {
        Some(s) => {
            Some(Kind::parse(s).map_err(|e| Error::validation(e.to_string(), Some("kind")))?)
        }
        None => None,
    };
    let status = match nonempty(&q.status) {
        Some(s) => Some(
            wicket_mod_items::Status::parse(s)
                .map_err(|e| Error::validation(e.to_string(), Some("status")))?,
        ),
        None => None,
    };
    let filter = wicket_mod_items::ListFilter {
        kind,
        status,
        number_prefix: nonempty(&q.number_prefix).map(str::to_string),
        cursor,
        limit,
    };
    let page = wicket_mod_items::list(state.pool(), filter)
        .await
        .map_err(map_items_err)?;
    let data: Vec<wicket_mod_items::api::ItemBody> = page
        .data
        .iter()
        .map(wicket_mod_items::api::ItemBody::from)
        .collect();
    Ok(serde_json::to_value(&crate::envelope::ListBody {
        data,
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    })?)
}

/// GET /api/v1/items/resolve
pub async fn resolve_item(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<ResolveNumberQ>,
) -> Response {
    let request_id = rid(&headers);
    match resolve_item_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn resolve_item_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: ResolveNumberQ,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "items.view").await?;
    let number = nonempty(&q.number)
        .ok_or_else(|| Error::validation("number is required", Some("number")))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "items.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let id = wicket_mod_items::resolve(&mut tx, number).await;
    tx.rollback().await?;
    let id = id.map_err(map_items_err)?;
    Ok(serde_json::to_value(ResolveIdBody { id: id.to_string() })?)
}

/// PATCH `/api/v1/items/{id}` body.
#[derive(Deserialize, JsonSchema)]
pub struct ItemPatch {
    #[serde(default)]
    revision: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    stock_uom: Option<i64>,
    #[serde(default)]
    stock_scale: Option<i16>,
    #[serde(default)]
    residual_tolerance: Option<String>,
    #[serde(default)]
    cost_method: Option<String>,
    #[serde(default)]
    standard: Option<MoneyBody>,
}

/// PATCH /api/v1/items/{id}
pub async fn update_item(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match update_item_inner(&state, &headers, &request_id, &id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn update_item_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ItemPatch = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "items.edit").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let item_id = parse_uuid(id, "id", ItemId::from_uuid)?;
    let kind = match body.kind.or(body.r#type) {
        Some(s) => {
            Some(Kind::parse(&s).map_err(|e| Error::validation(e.to_string(), Some("kind")))?)
        }
        None => None,
    };
    let residual = match body.residual_tolerance.as_deref() {
        Some(s) => Some(
            s.parse()
                .map_err(|_| Error::validation("residual_tolerance", Some("residual_tolerance")))?,
        ),
        None => None,
    };
    let cost_method = match body.cost_method.as_deref() {
        Some(s) => Some(parse_cost_method(s)?),
        None => None,
    };
    let standard = match body.standard {
        Some(m) => Some(m.to_money()?),
        None => None,
    };
    let patch = UpdateItem {
        version: expected,
        revision: body.revision,
        description: body.description,
        kind,
        stock_uom: body.stock_uom.map(wicket_core::UnitId),
        stock_scale: body.stock_scale,
        residual_tolerance: residual,
        cost_method,
        standard,
    };
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "items.update",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let item = wicket_mod_items::update(&mut tx, item_id, patch)
        .await
        .map_err(map_items_err)?;
    let body = serde_json::to_value(wicket_mod_items::api::ItemBody::from(&item))?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

/// POST /api/v1/items/{id}/release
pub async fn release_item(
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
        release_item_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn release_item_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let session = extract::require_mutation(state, headers, request_id, "items.release").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let item_id = parse_uuid(id, "id", ItemId::from_uuid)?;
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_items::DOC_TYPE.into(),
        doc_id: Identifier::from_uuid(item_id.as_uuid()),
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc, "release");
    ctx.session_id = Some(session.id.to_string());
    ctx.request_id = Some(request_id.to_string());
    ctx.source_kind = "api".into();
    ctx.reason = Some("api".into());
    ctx.actor_display = Some(session.display_name.clone());
    let write = fresh_write(state).await?;
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let current = wicket_mod_items::get(state.pool(), item_id).await?;
    check_version(current.version, expected)?;
    let item = wicket_mod_items::release(&mut tx, state.kernel(), &ctx, item_id).await?;
    let body = serde_json::to_value(wicket_mod_items::api::ItemBody::from(&item))?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}
