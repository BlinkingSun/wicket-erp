//! Inventory HTTP handlers.

use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wicket_core::{Identifier, ItemId, LocationId, LotId};
use wicket_db::Tx;
use wicket_mod_inventory::{BalanceQuery, DocumentKind, LineInput, ReceiveRequest, ReleaseRequest};

use super::{
    H, IntoActor, ReceiptLine, bind_esign_header, blocking, fresh_write, json_status, line_input,
    parse_json, required_edge_token, rid,
};
use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract::{self, check_version, require_if_match};
use crate::idempotency;
use crate::wire::{MoneyBody, QuantityBody, parse_uuid};

use axum::body::Bytes;

/// GET `/api/v1/inventory/on-hand` body. Amounts are decimal strings, not `AnyQuantity`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct OnHandBody {
    on_hand: String,
    available: String,
}

#[derive(Deserialize)]
pub struct ReceiptBody {
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    lot_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    purchase_order: Option<String>,
    #[serde(default)]
    quantity: Option<QuantityBody>,
    #[serde(default)]
    entered: Option<QuantityBody>,
    #[serde(default)]
    unit_cost: Option<MoneyBody>,
    #[serde(default)]
    lines: Option<Vec<ReceiptLine>>,
    #[serde(default)]
    actor_id: Option<String>,
}

/// POST /api/v1/inventory/receipts
pub async fn create_receipt(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    let state2 = state.clone();
    let headers2 = headers.clone();
    let rid2 = request_id.clone();
    let raw = body.to_vec();
    match blocking(&request_id, move || async move {
        receipt_inner(&state2, &headers2, &rid2, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn receipt_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ReceiptBody = parse_json(raw)?;
    if body.actor_id.is_some() {
        return Err(Error::validation(
            "actor_id is not accepted",
            Some("actor_id"),
        ));
    }
    let session =
        extract::require_mutation(state, headers, request_id, "inventory.receive").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let to_location = match body.location_id {
        Some(s) => parse_uuid(&s, "location_id", LocationId::from_uuid)?,
        None => {
            return Err(Error::validation(
                "location_id required",
                Some("location_id"),
            ));
        }
    };
    let mut lines = Vec::new();
    if let Some(extra) = body.lines {
        for l in extra {
            lines.push(line_input(&l)?);
        }
    } else {
        let item = body
            .item_id
            .as_deref()
            .ok_or_else(|| Error::validation("item_id required", Some("item_id")))?;
        let entered = body
            .entered
            .as_ref()
            .or(body.quantity.as_ref())
            .ok_or_else(|| Error::validation("quantity required", Some("quantity")))?
            .to_qty()?;
        lines.push(LineInput {
            item: parse_uuid(item, "item_id", ItemId::from_uuid)?,
            entered,
            lot: match body.lot_id {
                Some(s) => Some(parse_uuid(&s, "lot_id", LotId::from_uuid)?),
                None => None,
            },
            serial: None,
            from_location: None,
            to_location: Some(to_location),
            package: None,
            amount: match body.unit_cost {
                Some(c) => {
                    let unit = c.to_money()?;
                    let qty = entered.amount;
                    Some(
                        wicket_core::Money::new(unit.amount() * qty, unit.currency())
                            .map_err(|e| Error::validation(e.to_string(), Some("unit_cost")))?,
                    )
                }
                None => None,
            },
            reason_code: None,
        });
    }
    let req = ReceiveRequest {
        to_location,
        reference: body.purchase_order,
        lines,
        expected: None,
        tolerance: None,
        idempotency_key: Some(key),
    };
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_inventory::DOC_TYPE.into(),
        doc_id: Identifier::generate(),
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc, "receive");
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
    let posted = wicket_mod_inventory::receive(&mut tx, state.kernel(), &ctx, req).await?;
    let body = serde_json::to_value(wicket_mod_inventory::DocumentBody::from(&posted))?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

#[derive(Deserialize)]
pub struct ReleaseInvBody {
    lot_id: String,
    from_location_id: String,
    to_location_id: String,
    entered: QuantityBody,
    #[serde(default)]
    amount: Option<MoneyBody>,
}

/// POST /api/v1/inventory/releases
pub async fn release_stock(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    let state2 = state.clone();
    let headers2 = headers.clone();
    let rid2 = request_id.clone();
    let raw = body.to_vec();
    match blocking(&request_id, move || async move {
        release_stock_inner(&state2, &headers2, &rid2, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn release_stock_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ReleaseInvBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "lots.release").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let lot = parse_uuid(&body.lot_id, "lot_id", LotId::from_uuid)?;
    let req = ReleaseRequest {
        lot,
        from_location: parse_uuid(
            &body.from_location_id,
            "from_location_id",
            LocationId::from_uuid,
        )?,
        to_location: parse_uuid(
            &body.to_location_id,
            "to_location_id",
            LocationId::from_uuid,
        )?,
        entered: body.entered.to_qty()?,
        amount: match body.amount {
            Some(m) => Some(m.to_money()?),
            None => None,
        },
        idempotency_key: Some(key),
    };
    // release_from_quarantine posts the movement then set_status (action lot.release).
    let doc = wicket_statemachine::DocRef {
        doc_type: "lot".into(),
        doc_id: Identifier::from_uuid(lot.as_uuid()),
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
    bind_esign_header(&mut ctx, headers);
    let write = fresh_write(state).await?;
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let current = wicket_mod_lots::get_lot(&mut tx, lot).await?;
    check_version(current.version, expected)?;
    let _ = required_edge_token(
        state,
        &mut tx,
        headers,
        session.principal.0.into_actor(),
        "Lot released",
        Identifier::from_uuid(lot.as_uuid()),
        current.version,
    )
    .await?;
    let posted =
        wicket_mod_inventory::release_from_quarantine(&mut tx, state.kernel(), &ctx, req).await?;
    let body = serde_json::to_value(wicket_mod_inventory::DocumentBody::from(&posted))?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

#[derive(Deserialize)]
pub struct OnHandQ {
    item_id: String,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    lot_id: Option<String>,
}

/// GET /api/v1/inventory/on-hand
pub async fn on_hand(
    State(state): State<AppState>,
    headers: H,

    Query(q): Query<OnHandQ>,
) -> Response {
    let request_id = rid(&headers);
    match on_hand_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn on_hand_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: OnHandQ,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "inventory.view").await?;
    let item = parse_uuid(&q.item_id, "item_id", ItemId::from_uuid)?;
    let location = match q.location_id {
        Some(s) => Some(parse_uuid(&s, "location_id", LocationId::from_uuid)?),
        None => None,
    };
    let lot = match q.lot_id {
        Some(s) => Some(parse_uuid(&s, "lot_id", LotId::from_uuid)?),
        None => None,
    };
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "inventory.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let query = BalanceQuery {
        item,
        location,
        lot,
    };
    let qty = wicket_mod_inventory::on_hand(&mut tx, query).await;
    let avail = wicket_mod_inventory::available(&mut tx, query).await;
    tx.rollback().await?;
    Ok(serde_json::to_value(&OnHandBody {
        on_hand: qty?.to_string(),
        available: avail?.to_string(),
    })?)
}

#[derive(Deserialize)]
struct CountBody {
    location_id: String,
    #[serde(default)]
    reference: Option<String>,
    lines: Vec<CountLineBody>,
    #[serde(default)]
    tolerance: Option<String>,
}

#[derive(Deserialize)]
struct CountLineBody {
    item_id: String,
    #[serde(default)]
    lot_id: Option<String>,
    counted: QuantityBody,
    expected: QuantityBody,
}

/// POST /api/v1/inventory/counts
pub async fn create_count(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    let state2 = state.clone();
    let headers2 = headers.clone();
    let rid2 = request_id.clone();
    let raw = body.to_vec();
    match blocking(&request_id, move || async move {
        count_inner(&state2, &headers2, &rid2, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn count_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: CountBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "inventory.count").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let location = parse_uuid(&body.location_id, "location_id", LocationId::from_uuid)?;
    let mut lines = Vec::new();
    for l in body.lines {
        lines.push(wicket_mod_inventory::CountLine {
            item: parse_uuid(&l.item_id, "item_id", ItemId::from_uuid)?,
            lot: match l.lot_id {
                Some(s) => Some(parse_uuid(&s, "lot_id", LotId::from_uuid)?),
                None => None,
            },
            serial: None,
            counted: l.counted.to_qty()?,
            expected: l.expected.to_qty()?,
            amount: None,
        });
    }
    let tolerance: rust_decimal::Decimal = body
        .tolerance
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| Error::validation("tolerance", Some("tolerance")))?;
    let req = wicket_mod_inventory::CountRequest {
        location,
        reference: body.reference,
        lines,
        tolerance,
        idempotency_key: Some(key),
    };
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_inventory::DOC_TYPE.into(),
        doc_id: Identifier::generate(),
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc, "count");
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
    let posted = wicket_mod_inventory::cycle_count(&mut tx, state.kernel(), &ctx, req).await?;
    let body = serde_json::to_value(wicket_mod_inventory::DocumentBody::from(&posted))?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

#[derive(Deserialize)]
pub struct ReversalBody {
    document_id: String,
    reason: String,
}

/// POST /api/v1/inventory/reversals
///
/// One `Tx::begin` / one `WriteContext` (R-2s-7): `reverse_posted_issue` posts
/// the ledger `REVERSAL` inside the request transaction; the GUC is never rebound.
pub async fn create_reversal(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    let state2 = state.clone();
    let headers2 = headers.clone();
    let rid2 = request_id.clone();
    let raw = body.to_vec();
    match blocking(&request_id, move || async move {
        reverse_inner(&state2, &headers2, &rid2, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn reverse_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ReversalBody = parse_json(raw)?;
    if body.reason.trim().is_empty() {
        return Err(Error::validation("reason is required", Some("reason")));
    }
    let session = extract::require_mutation(state, headers, request_id, "inventory.adjust").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let issue_id = parse_uuid(&body.document_id, "document_id", Identifier::from_uuid)?;
    let doc_ref = wicket_statemachine::DocRef {
        doc_type: wicket_mod_inventory::DOC_TYPE.into(),
        doc_id: issue_id,
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc_ref, "void");
    ctx.session_id = Some(session.id.to_string());
    ctx.request_id = Some(request_id.to_string());
    ctx.source_kind = "api".into();
    ctx.reason = Some(body.reason.clone());
    ctx.actor_display = Some(session.display_name.clone());
    let write = fresh_write(state).await?;
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let current = wicket_mod_inventory::load_document(&mut tx, issue_id).await?;
    if current.kind != DocumentKind::Issue {
        return Err(Error::conflict(
            "only issue documents can be reversed",
            Some("document_id"),
        ));
    }
    if current.posted_group_id.is_none() {
        return Err(Error::conflict(
            "document is not posted",
            Some("document_id"),
        ));
    }
    let reversal_group =
        wicket_mod_inventory::reverse_posted_issue(&mut tx, state.kernel(), &ctx, issue_id).await?;
    let posted = wicket_mod_inventory::load_document(&mut tx, issue_id).await?;
    let mut payload = serde_json::to_value(wicket_mod_inventory::DocumentBody::from(&posted))?;
    payload["reversal_group_id"] = json!(reversal_group.to_string());
    payload["reason"] = json!(body.reason);
    idempotency::remember(&mut tx, key, &hash, 201, &payload).await?;
    tx.commit().await?;
    Ok((201, payload))
}

/// Health.
#[allow(dead_code)]
fn _health_marker() {}
