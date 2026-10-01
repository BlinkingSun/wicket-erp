//! HTTP handlers. Mutations open `Tx::begin` only after a session is present.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use wicket_core::{
    Identifier, ItemId, LotId, RecordRef, SignatureError, SignatureId, SignatureMeaning,
    SignatureToken,
};
use wicket_db::{ReadPool, Tx, WriteContext, WritePool};
use wicket_mod_inventory::LineInput;

use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract::{self, require_if_match};
use crate::idempotency;
use crate::session;
use crate::wire::{MoneyBody, QuantityBody, parse_uuid};

use axum::body::Bytes;

type H = HeaderMap;

fn rid(headers: &H) -> String {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("missing")
        .to_string()
}

fn parse_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).map_err(|e| Error::validation(e.to_string(), None))
}

fn nonempty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn parse_limit(raw: Option<&str>) -> Result<Option<u32>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let n: u32 = s
        .parse()
        .map_err(|_| Error::validation("limit must be an integer", Some("limit")))?;
    if !(1..=200).contains(&n) {
        return Err(Error::validation(
            "limit must be between 1 and 200",
            Some("limit"),
        ));
    }
    Ok(Some(n))
}

fn json_status(status: u16, v: Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(v),
    )
        .into_response()
}

/// GET `/api/v1/items/resolve` and `/api/v1/work-orders/resolve` body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ResolveIdBody {
    id: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ResolveNumberQ {
    #[serde(default)]
    number: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListQ {
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct ReceiptLine {
    item_id: String,
    #[serde(default)]
    lot_id: Option<String>,
    #[serde(default)]
    package_id: Option<String>,
    #[serde(default)]
    quantity: Option<QuantityBody>,
    #[serde(default)]
    entered: Option<QuantityBody>,
    #[serde(default)]
    amount: Option<MoneyBody>,
}

fn line_input(l: &ReceiptLine) -> Result<LineInput> {
    let entered = if let Some(q) = l.entered.as_ref().or(l.quantity.as_ref()) {
        q.to_qty()?
    } else if l.package_id.is_some() {
        wicket_core::AnyQuantity {
            amount: rust_decimal::Decimal::ZERO,
            unit: wicket_core::UnitId(1),
            dimension: wicket_core::DimensionKind::Count,
        }
    } else {
        return Err(Error::validation("quantity required", Some("quantity")));
    };
    Ok(LineInput {
        item: parse_uuid(&l.item_id, "item_id", ItemId::from_uuid)?,
        entered,
        lot: match &l.lot_id {
            Some(s) => Some(parse_uuid(s, "lot_id", LotId::from_uuid)?),
            None => None,
        },
        serial: None,
        from_location: None,
        to_location: None,
        package: match &l.package_id {
            Some(s) => Some(parse_uuid(s, "package_id", |u| {
                wicket_mod_lots::PackageId::from_uuid(u)
            })?),
            None => None,
        },
        amount: match &l.amount {
            Some(m) => Some(m.to_money()?),
            None => None,
        },
        reason_code: None,
    })
}

#[allow(clippy::result_large_err)]
fn blocking<T, F, Fut>(request_id: &str, f: F) -> std::result::Result<T, Response>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T>>,
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::Config(format!("runtime: {e}")))?;
            rt.block_on(f())
        })();
        let _ = tx.send(result);
    });
    match rx.recv() {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(error_response(e, request_id)),
        Err(e) => Err(error_response(Error::Config(format!("{e}")), request_id)),
    }
}

async fn fresh_write(state: &AppState) -> Result<WritePool> {
    Ok(WritePool::new(
        wicket_db::connect(state.database_url()).await?,
    ))
}

trait IntoActor {
    fn into_actor(self) -> wicket_core::Actor;
}

impl IntoActor for wicket_core::Identifier {
    fn into_actor(self) -> wicket_core::Actor {
        wicket_core::Actor {
            id: self,
            kind: wicket_core::ActorKind::User,
        }
    }
}

fn dummy_signature_token(
    actor: wicket_core::Actor,
    meaning: &str,
    record_id: Identifier,
    version: i64,
    signature: SignatureId,
) -> SignatureToken {
    SignatureToken {
        signature,
        signer: actor,
        meaning: SignatureMeaning(meaning.into()),
        record: RecordRef {
            table: "sm.instance".into(),
            id: record_id,
            version,
        },
        record_content_hash: [0; 32],
    }
}

/// Stamp `WriteContext.esign_id` from `X-Wicket-Signature` before `Tx::begin`
/// (D-2b-1). Never a nil UUID (D-2b-5 missing-token).
fn bind_esign_header(ctx: &mut WriteContext, headers: &H) {
    if let Some(raw) = headers
        .get("x-wicket-signature")
        .and_then(|v| v.to_str().ok())
        && let Ok(id) = Uuid::parse_str(raw)
        && !id.is_nil()
    {
        ctx.esign_id = Some(id.to_string());
    }
}

/// Pre-transition Consumed check (w2b-c1-server). Reads `consumed_at`
/// through the published esign seam — never `SELECT esign.*` here (R-2s-3).
async fn signature_row_consumed(tx: &mut Tx<'_>, id: SignatureId) -> Result<bool> {
    Ok(wicket_esign::signature_consumed_at(tx, id).await?.is_some())
}

fn consumed_conflict() -> Error {
    Error::http(
        "CONFLICT",
        SignatureError::Consumed.to_string(),
        None,
        StatusCode::CONFLICT,
    )
}

/// Bound-edge token: `X-Wicket-Signature` when present, else `None` so the
/// executor reports `Invalid("missing token")` (D-2b-5). A consumed row is
/// 409 before the state-machine edge runs.
async fn required_edge_token(
    state: &AppState,
    tx: &mut Tx<'_>,
    headers: &H,
    actor: wicket_core::Actor,
    meaning: &str,
    record_id: Identifier,
    version: i64,
) -> Result<Option<SignatureToken>> {
    let Some(raw) = headers
        .get("x-wicket-signature")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    let Ok(id) = Uuid::parse_str(raw) else {
        return Ok(None);
    };
    if id.is_nil() {
        return Ok(None);
    }
    let sid = SignatureId::from_uuid(id);
    if signature_row_consumed(tx, sid).await? {
        return Err(consumed_conflict());
    }
    if let Some(tok) = state.kernel().load_signature_token(tx, sid).await? {
        return Ok(Some(tok));
    }
    Ok(Some(dummy_signature_token(
        actor, meaning, record_id, version, sid,
    )))
}

/// Mint response is the crate D-2b-2 wire (including live-version supersession).
async fn manifestation_via_tx(tx: &mut Tx<'_>, id: SignatureId) -> Result<Value> {
    let body = wicket_esign::manifestation_in_tx(tx, id).await?;
    Ok(serde_json::to_value(body)?)
}

/// POST /api/v1/calibration/certificates/{id}/approve — item 7 probe.
pub async fn approve_calibration(
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
        approve_cal_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn approve_cal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let session =
        extract::require_mutation(state, headers, request_id, "calibration.approve").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let expected = require_if_match(headers)?;
    let _ = expected;
    let doc_id = parse_uuid(id, "id", Identifier::from_uuid)?;
    let doc = wicket_statemachine::DocRef {
        doc_type: "calibration.certificate".into(),
        doc_id,
    };
    let mut ctx =
        state
            .kernel()
            .transition_context(session.principal.0.into_actor(), &doc, "approve");
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
    // Required edge: `X-Wicket-Signature` or `None` (D-2b-5 missing token).
    let token = required_edge_token(
        state,
        &mut tx,
        headers,
        session.principal.0.into_actor(),
        "Approved",
        doc_id,
        1,
    )
    .await?;
    state
        .kernel()
        .transition(&mut tx, &doc, "approve", token.as_ref(), &ctx)
        .await?;
    let body = serde_json::to_value(esign::CalibrationApprovedBody {
        id: doc_id.to_string(),
        status: "approved".into(),
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &body).await?;
    tx.commit().await?;
    Ok((200, body))
}

/// Permission keys to snapshot at mint, from the target record's Required
/// edges (`machine` × `meaning`). `permission_snapshot` is a set: two
/// Required edges that share a meaning on one machine both appear.
///
/// The static table is the fallback when the machine has no Required edge
/// for the meaning (plain-shop `NoSignatures`).
fn permission_for_meaning(
    profile: &wicket_module::Profile,
    machine: &str,
    meaning: &str,
) -> Vec<wicket_core::PermissionKey> {
    let keys = profile.required_edge_permission(machine, meaning);
    if !keys.is_empty() {
        return keys.into_iter().map(wicket_core::PermissionKey).collect();
    }
    vec![wicket_core::PermissionKey(
        match meaning {
            "Approved" => "calibration.approve",
            "Released" => "wo.release",
            "Lot released" => "lots.release",
            other => other,
        }
        .into(),
    )]
}

/// POST /api/v1/esign/challenges
pub async fn esign_challenge(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    match esign_challenge_inner(&state, &headers, &request_id).await {
        Ok(v) => json_status(200, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn esign_challenge_inner(state: &AppState, headers: &H, request_id: &str) -> Result<Value> {
    let session = extract::require_mutation(state, headers, request_id, "identity.session").await?;
    let write = fresh_write(state).await?;
    let ctx = session::write_context(
        &session,
        "esign.challenge",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    let challenge = wicket_esign::challenge(
        &mut tx,
        session.principal,
        &state.kernel().profile.session_policy,
    )
    .await?;
    tx.commit().await?;
    Ok(serde_json::to_value(challenge)?)
}

/// POST /api/v1/esign/signatures
pub async fn esign_mint(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match esign_mint_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn esign_mint_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let session = extract::require_mutation(state, headers, request_id, "identity.session").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let body: esign::EsignMintBody = parse_json(raw)?;
    let write = fresh_write(state).await?;
    let ctx = session::write_context(
        &session,
        "esign.mint",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    let principal = wicket_identity::load_principal(state.pool(), session.principal).await?;
    let doc_id = parse_uuid(&body.record.id, "record.id", Identifier::from_uuid)?;
    let rec = RecordRef {
        table: body.record.table.clone(),
        id: doc_id,
        version: body.record.version,
    };
    let meaning = body.meaning.clone();
    let fallback_type = body
        .doc_type
        .clone()
        .unwrap_or_else(|| body.record.table.clone());
    let (doc_type, inst) = if body.record.table == "sm.instance" {
        match state.kernel().load_sm_instance(&mut tx, doc_id).await? {
            Some((live_type, state_name, version)) => (
                body.doc_type.unwrap_or(live_type.clone()),
                wicket_esign::InstanceTriple {
                    doc_type: live_type,
                    doc_id,
                    state: state_name,
                    version,
                },
            ),
            None => (
                fallback_type.clone(),
                wicket_esign::InstanceTriple {
                    doc_type: fallback_type.clone(),
                    doc_id,
                    state: String::new(),
                    version: body.record.version,
                },
            ),
        }
    } else {
        (
            fallback_type.clone(),
            wicket_esign::InstanceTriple {
                doc_type: fallback_type.clone(),
                doc_id,
                state: String::new(),
                version: body.record.version,
            },
        )
    };
    let mut components = Vec::new();
    if body
        .identification
        .code
        .as_ref()
        .is_some_and(|c| !c.is_empty())
    {
        components.push("code".into());
    }
    if body
        .identification
        .secret
        .as_ref()
        .is_some_and(|s| !s.is_empty())
    {
        components.push("secret".into());
    }
    let projection = state
        .kernel()
        .live_record(&mut tx, &doc_type, doc_id)
        .await?;
    // `required_edge_permission` returns a set (shared meaning → every
    // matching key). mint takes one PermissionKey; the first matching
    // Required-edge permission is snapshotted, plus meaning_policy's hint.
    let permission = permission_for_meaning(&state.kernel().profile, &inst.doc_type, &meaning)
        .into_iter()
        .next()
        .expect("permission_for_meaning never empty");
    let sig = wicket_esign::mint(
        &mut tx,
        &wicket_esign::MintRequest {
            components,
            code: body.identification.code.clone(),
            secret: body.identification.secret.clone().unwrap_or_default(),
            meaning: SignatureMeaning(meaning.clone()),
            reason: body.reason,
            record: rec,
            doc_type,
            projection,
            instance: inst,
            permission,
            signed_at_zone: state
                .kernel()
                .profile
                .seeded_permissions
                .display_timezone
                .clone(),
            policy: state.kernel().profile.session_policy.clone(),
            principal,
            login_session_id: Some(session.id),
            device_fingerprint: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string),
            source_ip: headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or(s).trim().to_string()),
            boot_epoch: "1".into(),
            credential_kind: "signing_password".into(),
        },
    )
    .await?;
    let sig_id = sig.id;
    let body = manifestation_via_tx(&mut tx, sig_id).await?;
    idempotency::remember(&mut tx, key, &hash, 201, &body).await?;
    tx.commit().await?;
    Ok((201, body))
}

/// GET /api/v1/esign/signatures/{id}
pub async fn esign_manifestation(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match esign_manifestation_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => json_status(200, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn esign_manifestation_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let _session =
        extract::require_permission(state, headers, request_id, "identity.session").await?;
    let sig_id = parse_uuid(id, "id", wicket_core::SignatureId::from_uuid)?;
    let body = wicket_esign::manifestation(&ReadPool::new(state.pool().clone()), sig_id).await?;
    Ok(serde_json::to_value(body)?)
}

/// GET /api/v1/esign/signatures/{id}/bundle
pub async fn esign_bundle(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match esign_bundle_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => json_status(200, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn esign_bundle_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let _session =
        extract::require_permission(state, headers, request_id, "esign.bundle.read").await?;
    let sig_id = parse_uuid(id, "id", wicket_core::SignatureId::from_uuid)?;
    let body = wicket_esign::archival_bundle(&ReadPool::new(state.pool().clone()), sig_id).await?;
    Ok(serde_json::to_value(body)?)
}

pub mod customfields;
pub mod documents;
pub mod esign;
pub mod identity;
pub mod inventory;
pub mod items;
pub mod kernel;
pub mod locations;
pub mod lots;
pub mod print;
pub mod production;

pub use inventory::OnHandBody;
pub use items::ItemPatch;
pub use kernel::{
    LoginBody, LoginResponse, NavigationBody, audit_export, health, login, logout, manifest,
    navigation, openapi,
};

pub use inventory::{create_count, create_receipt, create_reversal, on_hand, release_stock};
pub use items::{create_item, get_item, list_items, release_item, resolve_item, update_item};
pub use locations::{
    create_location, deactivate_location, get_location, list_location_tree, list_locations,
};
pub use lots::{
    create_lot, create_package, create_serials, get_lot, list_lots, list_packages, list_serials,
    set_lot_status,
};
pub use production::{
    complete_wo, create_wo, genealogy_trace, get_genealogy_job, get_impact, get_wo, issue_wo,
    list_work_orders, release_wo, resolve_work_order,
};
