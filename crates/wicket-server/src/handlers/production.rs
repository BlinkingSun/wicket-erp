//! Work-order and genealogy HTTP handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wicket_core::{Identifier, ItemId, LocationId, LotId};
use wicket_db::Tx;
use wicket_mod_production_min::{
    CompleteRequest, CreateWorkOrder, FinishedLotTemplate, IssueMaterialRequest, StartRequest,
};

use super::{
    H, IntoActor, ReceiptLine, ResolveIdBody, ResolveNumberQ, blocking, fresh_write, json_status,
    line_input, nonempty, parse_json, parse_limit, rid,
};
use crate::boot::AppState;
use crate::envelope::{ListBody, error_response};
use crate::error::{Error, Result};
use crate::extract::{self, check_version, require_if_match};
use crate::idempotency;
use crate::session::write_context;
use crate::wire::{MoneyBody, QuantityBody, parse_uuid};

use axum::body::Bytes;

fn map_production_err(e: wicket_mod_production_min::Error) -> Error {
    match e {
        wicket_mod_production_min::Error::InvalidLimit => {
            Error::validation("limit must be between 1 and 200", Some("limit"))
        }
        wicket_mod_production_min::Error::Manifest(_) => Error::validation(e.to_string(), None),
        other => other.into(),
    }
}

fn map_genealogy_err(e: wicket_mod_genealogy::Error) -> Error {
    match e {
        wicket_mod_genealogy::Error::Lots(wicket_mod_lots::Error::NotFound) => {
            Error::not_found("lot not found")
        }
        wicket_mod_genealogy::Error::NotFound => Error::not_found("not found"),
        other => other.into(),
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct WoCreate {
    item_id: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    quantity: QuantityBody,
    revision: String,
    #[serde(default)]
    id: Option<String>,
}

/// POST /api/v1/work-orders
pub async fn create_wo(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match create_wo_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_wo_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: WoCreate = parse_json(raw)?;
    if body.id.is_some() {
        return Err(Error::validation(
            "clients must not mint identifiers",
            Some("id"),
        ));
    }
    let session =
        extract::require_mutation(state, headers, request_id, "production.create").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "production.create",
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
    let wo = wicket_mod_production_min::create(
        &mut tx,
        state.kernel(),
        CreateWorkOrder {
            item: parse_uuid(&body.item_id, "item_id", ItemId::from_uuid)?,
            quantity_ordered: body.quantity.to_qty()?,
            revision: body.revision,
        },
    )
    .await?;
    let body = wo_json(&wo)?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// Wire body `wo_json` emits today. Not `wicket_mod_production_min::api::WorkOrderBody`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkOrderJson {
    id: String,
    number: Option<String>,
    item_id: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    quantity: QuantityBody,
    status: String,
    revision: String,
    wip_location_id: Option<String>,
    version: i64,
    application_version: String,
    configuration_version: String,
    released_at: Option<String>,
    completed_at: Option<String>,
}

impl WorkOrderJson {
    fn from_work_order(wo: &wicket_mod_production_min::WorkOrder) -> Self {
        Self {
            id: wo.id.to_string(),
            number: wo.number.clone(),
            item_id: wo.item.to_string(),
            quantity: QuantityBody::from_qty(&wo.quantity_ordered),
            status: wo.status.as_str().to_string(),
            revision: wo.revision.clone(),
            wip_location_id: wo.wip_location.map(|l| l.to_string()),
            version: wo.version,
            application_version: wo.application_version.clone(),
            configuration_version: wo.configuration_version.clone(),
            released_at: wo.released_at.map(|t| t.to_rfc3339()),
            completed_at: wo.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

fn wo_json(wo: &wicket_mod_production_min::WorkOrder) -> Result<Value> {
    Ok(serde_json::to_value(WorkOrderJson::from_work_order(wo))?)
}

/// GET /api/v1/work-orders/{id}
pub async fn get_wo(State(state): State<AppState>, headers: H, Path(id): Path<String>) -> Response {
    let request_id = rid(&headers);
    match get_wo_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_wo_inner(state: &AppState, headers: &H, request_id: &str, id: &str) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "production.view").await?;
    let wo_id = parse_uuid(id, "id", Identifier::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "production.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let wo = wicket_mod_production_min::load(&mut tx, wo_id).await;
    tx.rollback().await?;
    wo_json(&wo?)
}

#[derive(Debug, Default, Deserialize)]
pub struct WoListQ {
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// GET /api/v1/work-orders
pub async fn list_work_orders(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<WoListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_work_orders_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_work_orders_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: WoListQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "production.view").await?;
    let limit = parse_limit(nonempty(&q.limit))?;
    let cursor = match nonempty(&q.cursor) {
        Some(c) => Some(parse_uuid(c, "cursor", Identifier::from_uuid)?),
        None => None,
    };
    let status = match nonempty(&q.status) {
        Some(s) => Some(
            wicket_mod_production_min::Status::parse(s)
                .map_err(|e| Error::validation(e.to_string(), Some("status")))?,
        ),
        None => None,
    };
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "production.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_mod_production_min::list(
        &mut tx,
        wicket_mod_production_min::ListFilter {
            status,
            cursor,
            limit,
        },
    )
    .await;
    tx.rollback().await?;
    let page = page.map_err(map_production_err)?;
    Ok(serde_json::to_value(&ListBody {
        data: page
            .data
            .iter()
            .map(WorkOrderJson::from_work_order)
            .collect(),
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    })?)
}

/// GET /api/v1/work-orders/resolve
pub async fn resolve_work_order(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<ResolveNumberQ>,
) -> Response {
    let request_id = rid(&headers);
    match resolve_work_order_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn resolve_work_order_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: ResolveNumberQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "production.view").await?;
    let number = nonempty(&q.number)
        .ok_or_else(|| Error::validation("number is required", Some("number")))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "production.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let id = wicket_mod_production_min::resolve(&mut tx, number).await;
    tx.rollback().await?;
    let id = id.map_err(map_production_err)?;
    Ok(serde_json::to_value(ResolveIdBody { id: id.to_string() })?)
}

/// POST .../release
pub async fn release_wo(
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
        release_wo_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn release_wo_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let session =
        extract::require_mutation(state, headers, request_id, "production.release").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let wo_id = parse_uuid(id, "id", Identifier::from_uuid)?;
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_production_min::DOC_TYPE.into(),
        doc_id: wo_id,
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
    let current = wicket_mod_production_min::load(&mut tx, wo_id).await?;
    check_version(current.version, expected)?;
    let wo = wicket_mod_production_min::release(&mut tx, state.kernel(), &ctx, wo_id).await?;
    let body = wo_json(&wo)?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

/// Schema-only mirror of `ReceiptLine` (lives in `handlers/mod.rs`, not owned here).
#[allow(dead_code)]
#[derive(JsonSchema)]
struct WorkOrderIssueLine {
    item_id: String,
    lot_id: Option<String>,
    package_id: Option<String>,
    #[schemars(with = "Option<wicket_core::AnyQuantity>")]
    quantity: Option<QuantityBody>,
    #[schemars(with = "Option<wicket_core::AnyQuantity>")]
    entered: Option<QuantityBody>,
    amount: Option<MoneyBody>,
}

#[derive(Deserialize, JsonSchema)]
pub struct IssueBody {
    from_location_id: String,
    #[schemars(with = "Vec<WorkOrderIssueLine>")]
    lines: Vec<ReceiptLine>,
}

/// POST .../issue
pub async fn issue_wo(
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
        issue_wo_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn issue_wo_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: IssueBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "production.issue").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let wo_id = parse_uuid(id, "id", Identifier::from_uuid)?;
    let lines: Result<Vec<_>> = body.lines.iter().map(line_input).collect();
    let lines = lines?;
    let from_location = parse_uuid(
        &body.from_location_id,
        "from_location_id",
        LocationId::from_uuid,
    )?;
    let wo_doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_production_min::DOC_TYPE.into(),
        doc_id: wo_id,
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &wo_doc, "issue");
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
    let current = wicket_mod_production_min::load(&mut tx, wo_id).await?;
    check_version(current.version, expected)?;
    let wo = wicket_mod_production_min::start(
        &mut tx,
        state.kernel(),
        &ctx,
        StartRequest {
            work_order: wo_id,
            issue: Some(IssueMaterialRequest {
                work_order: wo_id,
                from_location,
                lines,
                idempotency_key: Some(key),
            }),
        },
    )
    .await?;
    let body = wo_json(&wo)?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

#[derive(Deserialize, JsonSchema)]
pub struct CompleteBody {
    #[schemars(with = "wicket_core::AnyQuantity")]
    quantity: QuantityBody,
    #[serde(default)]
    #[schemars(with = "Option<wicket_core::AnyQuantity>")]
    scrap: Option<QuantityBody>,
    #[serde(default)]
    finished_lot_number: Option<String>,
    location_id: String,
    #[serde(default)]
    serial_from: Option<String>,
    #[serde(default)]
    serial_template: Option<String>,
}

/// Nested `finished_lot` object on the complete response blob.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FinishedLotRef {
    id: String,
}

/// Wire body `complete_wo` emits today. Not `wicket_mod_production_min::api::CompletionBody`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkOrderCompleteJson {
    id: String,
    number: Option<String>,
    status: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    quantity: QuantityBody,
    finished_lot: FinishedLotRef,
    group_id: String,
    version: i64,
    posted_at: Option<String>,
    application_version: String,
    configuration_version: String,
}

/// POST .../complete
pub async fn complete_wo(
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
        complete_wo_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn complete_wo_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: CompleteBody = parse_json(raw)?;
    let session =
        extract::require_mutation(state, headers, request_id, "production.complete").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let wo_id = parse_uuid(id, "id", Identifier::from_uuid)?;
    let good = body.quantity.to_qty()?;
    let scrap = match body.scrap {
        Some(s) => s.to_qty()?,
        None => AnyQuantityZero::zero_like(&good),
    };
    let doc = wicket_statemachine::DocRef {
        doc_type: wicket_mod_production_min::DOC_TYPE.into(),
        doc_id: wo_id,
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc, "complete");
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
    let current = wicket_mod_production_min::load(&mut tx, wo_id).await?;
    check_version(current.version, expected)?;
    let completion = wicket_mod_production_min::complete(
        &mut tx,
        state.kernel(),
        &ctx,
        CompleteRequest {
            work_order: wo_id,
            to_location: parse_uuid(&body.location_id, "location_id", LocationId::from_uuid)?,
            good,
            scrap,
            finished_lot: FinishedLotTemplate {
                number: body.finished_lot_number,
                template: None,
                serial_template: body.serial_template.or(body.serial_from),
            },
        },
    )
    .await?;
    let wo = wicket_mod_production_min::load(&mut tx, wo_id).await?;
    let body = serde_json::to_value(&WorkOrderCompleteJson {
        id: wo.id.to_string(),
        number: wo.number.clone(),
        status: wo.status.as_str().to_string(),
        quantity: QuantityBody::from_qty(&wo.quantity_ordered),
        finished_lot: FinishedLotRef {
            id: completion.finished_lot.to_string(),
        },
        group_id: completion.group_id.to_string(),
        version: wo.version,
        posted_at: wo.completed_at.map(|t| t.to_rfc3339()),
        application_version: wo.application_version.clone(),
        configuration_version: wo.configuration_version.clone(),
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

struct AnyQuantityZero;
impl AnyQuantityZero {
    fn zero_like(q: &wicket_core::AnyQuantity) -> wicket_core::AnyQuantity {
        wicket_core::AnyQuantity {
            amount: Decimal::ZERO,
            unit: q.unit,
            dimension: q.dimension,
        }
    }
}

#[derive(Deserialize)]
pub struct TraceQ {
    #[serde(default)]
    from_lot_id: Option<String>,
    #[serde(default)]
    direction: Option<String>,
}

/// GET /api/v1/genealogy/trace
pub async fn genealogy_trace(
    State(state): State<AppState>,
    headers: H,

    Query(q): Query<TraceQ>,
) -> Response {
    let request_id = rid(&headers);
    match trace_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn trace_inner(state: &AppState, headers: &H, request_id: &str, q: TraceQ) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "genealogy.view").await?;
    let lot = q
        .from_lot_id
        .as_deref()
        .ok_or_else(|| Error::validation("from_lot_id required", Some("from_lot_id")))?;
    let lot_id = parse_uuid(lot, "from_lot_id", LotId::from_uuid)?;
    let direction = match q.direction.as_deref().unwrap_or("forward") {
        "forward" => wicket_mod_genealogy::Direction::Forward,
        "backward" => wicket_mod_genealogy::Direction::Backward,
        "both" => wicket_mod_genealogy::Direction::Both,
        other => {
            return Err(Error::validation(
                format!("bad direction {other}"),
                Some("direction"),
            ));
        }
    };
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "genealogy.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let outcome = wicket_mod_genealogy::trace(
        &mut tx,
        session.principal.0.into_actor(),
        wicket_mod_genealogy::TraceRequest {
            origin: wicket_mod_genealogy::TraceOrigin::Lot(lot_id),
            direction,
            depth: None,
            max_postings: None,
        },
    )
    .await;
    tx.rollback().await?;
    match outcome? {
        wicket_mod_genealogy::TraceOutcome::Inline(body) => Ok(serde_json::to_value(&body)?),
        wicket_mod_genealogy::TraceOutcome::Accepted { job_id, result_url } => {
            Ok(serde_json::to_value(&wicket_mod_genealogy::AcceptedBody {
                job_id: job_id.0.to_string(),
                result_url,
            })?)
        }
    }
}

/// GET /api/v1/genealogy/impact/{lot}
pub async fn get_impact(
    State(state): State<AppState>,
    headers: H,
    Path(lot): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_impact_inner(&state, &headers, &request_id, &lot).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_impact_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    lot: &str,
) -> Result<Value> {
    let session = extract::require_permission(state, headers, request_id, "genealogy.view").await?;
    let lot_id = parse_uuid(lot, "lot", LotId::from_uuid)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "genealogy.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let impact = wicket_mod_genealogy::impact(&mut tx, lot_id).await;
    tx.rollback().await?;
    serde_json::to_value(&impact.map_err(map_genealogy_err)?).map_err(Error::from)
}

/// GET /api/v1/genealogy/jobs/{id}
pub async fn get_genealogy_job(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_genealogy_job_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_genealogy_job_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let _session =
        extract::require_permission(state, headers, request_id, "genealogy.view").await?;
    let job_id = parse_uuid(id, "id", |u| wicket_jobs::JobId(Identifier::from_uuid(u)))?;
    match wicket_mod_genealogy::job_status(state.pool(), job_id)
        .await
        .map_err(map_genealogy_err)?
    {
        Some(status) => serde_json::to_value(&status).map_err(Error::from),
        None => Err(Error::not_found("job not found")),
    }
}

#[cfg(test)]
mod wire_freeze {
    use super::*;
    use serde_json::json;

    fn qty() -> QuantityBody {
        QuantityBody {
            amount: "5.00000000".into(),
            unit: 1,
            dimension: "Count".into(),
        }
    }

    #[test]
    fn work_order_json_matches_wo_json_blob() {
        let quantity = qty();
        let typed = WorkOrderJson {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            number: None,
            item_id: "01932c5a-8b10-7001-8000-000000000002".into(),
            quantity: quantity.clone(),
            status: "draft".into(),
            revision: "C".into(),
            wip_location_id: None,
            version: 1,
            application_version: "0.0.0-dev".into(),
            configuration_version: "plain-shop".into(),
            released_at: None,
            completed_at: None,
        };
        let blob = json!({
            "id": "01932c5a-8b10-7001-8000-000000000001",
            "number": null,
            "item_id": "01932c5a-8b10-7001-8000-000000000002",
            "quantity": quantity,
            "status": "draft",
            "revision": "C",
            "wip_location_id": null,
            "version": 1,
            "application_version": "0.0.0-dev",
            "configuration_version": "plain-shop",
            "released_at": null,
            "completed_at": null,
        });
        assert_eq!(serde_json::to_value(&typed).unwrap(), blob);
    }

    #[test]
    fn work_order_json_matches_populated_wo_json_blob() {
        let quantity = qty();
        let typed = WorkOrderJson {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            number: Some("WO-2026-0001".into()),
            item_id: "01932c5a-8b10-7001-8000-000000000002".into(),
            quantity: quantity.clone(),
            status: "released".into(),
            revision: "C".into(),
            wip_location_id: Some("01932c5a-8b10-7001-8000-000000000003".into()),
            version: 2,
            application_version: "0.0.0-dev".into(),
            configuration_version: "plain-shop".into(),
            released_at: Some("2026-09-19T12:00:00+00:00".into()),
            completed_at: None,
        };
        let blob = json!({
            "id": "01932c5a-8b10-7001-8000-000000000001",
            "number": "WO-2026-0001",
            "item_id": "01932c5a-8b10-7001-8000-000000000002",
            "quantity": quantity,
            "status": "released",
            "revision": "C",
            "wip_location_id": "01932c5a-8b10-7001-8000-000000000003",
            "version": 2,
            "application_version": "0.0.0-dev",
            "configuration_version": "plain-shop",
            "released_at": "2026-09-19T12:00:00+00:00",
            "completed_at": null,
        });
        assert_eq!(serde_json::to_value(&typed).unwrap(), blob);
    }

    #[test]
    fn list_envelope_matches_json_blob() {
        let quantity = qty();
        let row = WorkOrderJson {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            number: None,
            item_id: "01932c5a-8b10-7001-8000-000000000002".into(),
            quantity: quantity.clone(),
            status: "draft".into(),
            revision: "C".into(),
            wip_location_id: None,
            version: 1,
            application_version: "0.0.0-dev".into(),
            configuration_version: "plain-shop".into(),
            released_at: None,
            completed_at: None,
        };
        let typed = ListBody {
            data: vec![row.clone()],
            next_cursor: None,
            has_more: false,
        };
        let blob = json!({
            "data": [row],
            "next_cursor": null,
            "has_more": false,
        });
        assert_eq!(serde_json::to_value(&typed).unwrap(), blob);
    }

    #[test]
    fn complete_json_matches_complete_wo_blob() {
        let quantity = qty();
        let typed = WorkOrderCompleteJson {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            number: Some("WO-2026-0001".into()),
            status: "completed".into(),
            quantity: quantity.clone(),
            finished_lot: FinishedLotRef {
                id: "01932c5a-8b10-7001-8000-000000000004".into(),
            },
            group_id: "01932c5a-8b10-7001-8000-000000000005".into(),
            version: 4,
            posted_at: Some("2026-09-19T13:00:00+00:00".into()),
            application_version: "0.0.0-dev".into(),
            configuration_version: "plain-shop".into(),
        };
        let blob = json!({
            "id": "01932c5a-8b10-7001-8000-000000000001",
            "number": "WO-2026-0001",
            "status": "completed",
            "quantity": quantity,
            "finished_lot": {
                "id": "01932c5a-8b10-7001-8000-000000000004",
            },
            "group_id": "01932c5a-8b10-7001-8000-000000000005",
            "version": 4,
            "posted_at": "2026-09-19T13:00:00+00:00",
            "application_version": "0.0.0-dev",
            "configuration_version": "plain-shop",
        });
        assert_eq!(serde_json::to_value(&typed).unwrap(), blob);
    }
}
