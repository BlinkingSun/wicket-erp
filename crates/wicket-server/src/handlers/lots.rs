//! Lot, package, and serial HTTP handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// POST `/api/v1/lots/{id}/packages` body. `contained` is the AnyQuantity wire
/// (`docs/10` §3.1); the handler stores it as [`QuantityBody`] so amount stays a
/// decimal string. `label_ref` is accepted and stored, not echoed.
#[derive(Deserialize, JsonSchema)]
pub struct PackageCreate {
    level: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    contained: QuantityBody,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    label_ref: Option<String>,
}

/// POST `/api/v1/lots/{id}/packages` 201 body. Frozen from the handler `json!`
/// blob: field is `contained`, not `PackageBody.contained_quantity`; `label_ref`
/// is omitted even when the request supplied one.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PackageCreatedBody {
    id: String,
    lot_id: String,
    parent_id: Option<String>,
    level: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    contained: QuantityBody,
}

/// GET `/api/v1/lots/{id}/packages` item. Frozen from the handler `json!` blob:
/// omits `lot_id` and `label_ref`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PackageListItem {
    id: String,
    parent_id: Option<String>,
    level: String,
    #[schemars(with = "wicket_core::AnyQuantity")]
    contained: QuantityBody,
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
    let body = serde_json::to_value(&PackageCreatedBody {
        id: pkg.id.as_uuid().to_string(),
        lot_id: pkg.lot.to_string(),
        parent_id: pkg.parent.map(|p| p.as_uuid().to_string()),
        level: pkg.level.as_str().to_string(),
        contained: QuantityBody::from_qty(&pkg.contained),
    })?;
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
    let data: Vec<PackageListItem> = tree
        .iter()
        .map(|p| PackageListItem {
            id: p.id.as_uuid().to_string(),
            parent_id: p.parent.map(|x| x.as_uuid().to_string()),
            level: p.level.as_str().to_string(),
            contained: QuantityBody::from_qty(&p.contained),
        })
        .collect();
    serde_json::to_value(&crate::envelope::ListBody {
        data,
        next_cursor: None,
        has_more: false,
    })
    .map_err(Error::from)
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

#[cfg(test)]
mod package_wire {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_package_named_type_matches_blob() {
        let contained = QuantityBody {
            amount: "24".into(),
            unit: 1,
            dimension: "Count".into(),
        };
        let blob = json!({
            "id": "01932c5a-8b10-7001-8000-000000000001",
            "lot_id": "01932c5a-8b10-7000-8000-000000000002",
            "parent_id": serde_json::Value::Null,
            "level": "case",
            "contained": contained.clone(),
        });
        let typed = serde_json::to_value(&PackageCreatedBody {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            lot_id: "01932c5a-8b10-7000-8000-000000000002".into(),
            parent_id: None,
            level: "case".into(),
            contained,
        })
        .expect("serialize");
        assert_eq!(blob, typed);
    }

    #[test]
    fn list_packages_named_type_matches_blob() {
        let contained = QuantityBody {
            amount: "24.00000000".into(),
            unit: 1,
            dimension: "Count".into(),
        };
        let item_blob = json!({
            "id": "01932c5a-8b10-7001-8000-000000000001",
            "parent_id": serde_json::Value::Null,
            "level": "case",
            "contained": contained.clone(),
        });
        let item = PackageListItem {
            id: "01932c5a-8b10-7001-8000-000000000001".into(),
            parent_id: None,
            level: "case".into(),
            contained,
        };
        assert_eq!(item_blob, serde_json::to_value(&item).expect("item"));
        let envelope_blob = json!({
            "data": [item_blob],
            "next_cursor": serde_json::Value::Null,
            "has_more": false
        });
        let typed = serde_json::to_value(&crate::envelope::ListBody {
            data: vec![item],
            next_cursor: None,
            has_more: false,
        })
        .expect("envelope");
        assert_eq!(envelope_blob, typed);
    }

    fn object_schema<T: schemars::JsonSchema>() -> serde_json::Value {
        let mut value = serde_json::to_value(schemars::schema_for!(T)).expect("schema");
        if value.get("properties").is_none() {
            let name = T::schema_name();
            if let Some(defn) = value.get("definitions").and_then(|d| d.get(&name)).cloned() {
                value = defn;
            }
        }
        value
    }

    #[test]
    fn package_schema_matches_frozen_blob_keys() {
        let created = object_schema::<PackageCreatedBody>();
        let mut created_props: Vec<_> = created["properties"]
            .as_object()
            .expect("created properties")
            .keys()
            .cloned()
            .collect();
        created_props.sort();
        assert_eq!(
            created_props,
            ["contained", "id", "level", "lot_id", "parent_id"]
        );
        assert!(created["properties"].get("label_ref").is_none());
        assert!(created["properties"].get("contained_quantity").is_none());
        let contained = &created["properties"]["contained"];
        let contained_ref = contained["$ref"]
            .as_str()
            .or_else(|| contained["allOf"][0]["$ref"].as_str())
            .unwrap_or_else(|| panic!("contained schema: {contained}"));
        assert!(contained_ref.ends_with("/AnyQuantity"), "{contained_ref}");

        let item = object_schema::<PackageListItem>();
        let mut item_props: Vec<_> = item["properties"]
            .as_object()
            .expect("item properties")
            .keys()
            .cloned()
            .collect();
        item_props.sort();
        assert_eq!(item_props, ["contained", "id", "level", "parent_id"]);
        assert!(item["properties"].get("lot_id").is_none());
        assert!(item["properties"].get("label_ref").is_none());
    }
}
