// w3b:customfields

//! HTTP handlers for `wicket-customfields` published APIs.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use uuid::Uuid;
use wicket_core::{Identifier, ItemId};
use wicket_customfields::{
    DatePrecision, Definition, DefinitionId, DefinitionSpec, DefinitionStatus, FieldType, Value,
    ValueWire,
};
use wicket_db::Tx;
use wicket_statemachine::Engine;

use crate::boot::AppState;
use crate::envelope::{ListBody, error_response};
use crate::error::{Error, Result};
use crate::extract::{self, require_if_match};
use crate::idempotency;
use crate::session::write_context;
use crate::wire::parse_uuid;

use axum::body::Bytes;

type H = HeaderMap;

/// Map this crate's errors (NotFound / state-machine / validation) for
/// [`crate::error::Error::envelope`].
pub fn envelope_arm(
    e: &wicket_customfields::Error,
) -> (&'static str, axum::http::StatusCode, Option<&str>, String) {
    match e {
        wicket_customfields::Error::NotFound => (
            "NOT_FOUND",
            axum::http::StatusCode::NOT_FOUND,
            None,
            e.to_string(),
        ),
        wicket_customfields::Error::StateMachine(sm) => match sm {
            wicket_statemachine::Error::PermissionDenied { .. } => (
                "FORBIDDEN",
                axum::http::StatusCode::FORBIDDEN,
                None,
                e.to_string(),
            ),
            wicket_statemachine::Error::ActionMismatch { .. } => (
                "VALIDATION",
                axum::http::StatusCode::BAD_REQUEST,
                Some("action"),
                e.to_string(),
            ),
            _ => (
                "INTERNAL",
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                None,
                e.to_string(),
            ),
        },
        wicket_customfields::Error::UnknownValidationRule { .. }
        | wicket_customfields::Error::ValidationFailed { .. }
        | wicket_customfields::Error::RequiredFieldMissing { .. }
        | wicket_customfields::Error::TypeMismatch { .. }
        | wicket_customfields::Error::TypeChangeRefused { .. }
        | wicket_customfields::Error::AuditRefused
        | wicket_customfields::Error::UnknownProfile(_) => (
            "VALIDATION",
            axum::http::StatusCode::BAD_REQUEST,
            None,
            e.to_string(),
        ),
        wicket_customfields::Error::RetireForbidden { .. } => (
            "FORBIDDEN",
            axum::http::StatusCode::FORBIDDEN,
            None,
            e.to_string(),
        ),
        wicket_customfields::Error::Retired | wicket_customfields::Error::AlreadyRetired => (
            "CONFLICT",
            axum::http::StatusCode::CONFLICT,
            None,
            e.to_string(),
        ),
        _ => (
            "INTERNAL",
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            None,
            e.to_string(),
        ),
    }
}

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

fn json_status(status: u16, v: JsonValue) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(v),
    )
        .into_response()
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

fn definition_id(u: Uuid) -> DefinitionId {
    DefinitionId::new(Identifier::from_uuid(u))
}

fn record_id(item: ItemId) -> Identifier {
    Identifier::from_uuid(item.as_uuid())
}

fn parse_field_type(s: &str) -> Result<FieldType> {
    FieldType::parse(s).ok_or_else(|| Error::validation(format!("unknown type {s}"), Some("type")))
}

fn definition_engine(profile: &str) -> Result<Engine> {
    let mut engine = Engine::new();
    engine.register_machine(wicket_customfields::definition_machine(profile)?)?;
    engine.freeze()?;
    Ok(engine)
}

fn value_payload(value: &Value) -> JsonValue {
    match value {
        Value::String(s) | Value::Text(s) | Value::Enum(s) => json!(s),
        Value::Integer(n) => json!(n),
        Value::Decimal { value, scale } => json!({"value": value.to_string(), "scale": scale}),
        Value::Bool(b) => json!(b),
        Value::Date { date, precision } => {
            json!({"value": date.to_string(), "precision": precision.as_str()})
        }
        Value::Reference { entity, id } => json!({"entity": entity, "id": id.to_string()}),
        _ => JsonValue::Null,
    }
}

fn wire_from_def(def: &Definition, value: Option<&Value>) -> ValueWire {
    ValueWire {
        key: def.key.clone(),
        type_name: def.field_type.as_str().to_string(),
        value: value.map(value_payload).unwrap_or(JsonValue::Null),
        definition_version: def.version,
    }
}

fn parse_typed_value(type_name: &str, raw: &JsonValue) -> Result<Value> {
    match parse_field_type(type_name)? {
        FieldType::String => Ok(Value::String(
            raw.as_str()
                .ok_or_else(|| Error::validation("string value required", Some("value")))?
                .to_string(),
        )),
        FieldType::Text => Ok(Value::Text(
            raw.as_str()
                .ok_or_else(|| Error::validation("text value required", Some("value")))?
                .to_string(),
        )),
        FieldType::Integer => {
            let n = raw
                .as_i64()
                .ok_or_else(|| Error::validation("integer value required", Some("value")))?;
            Ok(Value::Integer(n))
        }
        FieldType::Decimal => {
            let obj = raw
                .as_object()
                .ok_or_else(|| Error::validation("decimal value object required", Some("value")))?;
            let value_s = obj
                .get("value")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| Error::validation("decimal value required", Some("value")))?;
            let scale = obj
                .get("scale")
                .and_then(JsonValue::as_i64)
                .ok_or_else(|| Error::validation("decimal scale required", Some("value")))?;
            let value: rust_decimal::Decimal = value_s
                .parse()
                .map_err(|_| Error::validation("decimal value", Some("value")))?;
            let scale = i16::try_from(scale)
                .map_err(|_| Error::validation("decimal scale", Some("value")))?;
            Ok(Value::Decimal { value, scale })
        }
        FieldType::Bool => {
            let b = raw
                .as_bool()
                .ok_or_else(|| Error::validation("bool value required", Some("value")))?;
            Ok(Value::Bool(b))
        }
        FieldType::Date => {
            let obj = raw
                .as_object()
                .ok_or_else(|| Error::validation("date value object required", Some("value")))?;
            let value_s = obj
                .get("value")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| Error::validation("date value required", Some("value")))?;
            let precision_s = obj
                .get("precision")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| Error::validation("date precision required", Some("value")))?;
            let date = chrono::NaiveDate::parse_from_str(value_s, "%Y-%m-%d")
                .map_err(|_| Error::validation("date value must be YYYY-MM-DD", Some("value")))?;
            let precision = match precision_s {
                "day" => DatePrecision::Day,
                "month" => DatePrecision::Month,
                "year" => DatePrecision::Year,
                other => {
                    return Err(Error::validation(
                        format!("unknown precision {other}"),
                        Some("value"),
                    ));
                }
            };
            Ok(Value::Date { date, precision })
        }
        FieldType::Enum => Ok(Value::Enum(
            raw.as_str()
                .ok_or_else(|| Error::validation("enum value required", Some("value")))?
                .to_string(),
        )),
        FieldType::Reference => {
            let obj = raw.as_object().ok_or_else(|| {
                Error::validation("reference value object required", Some("value"))
            })?;
            let entity = obj
                .get("entity")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| Error::validation("reference entity required", Some("value")))?
                .to_string();
            let id_s = obj
                .get("id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| Error::validation("reference id required", Some("value")))?;
            let id = parse_uuid(id_s, "value.id", Identifier::from_uuid)?;
            Ok(Value::Reference { entity, id })
        }
        _ => Err(Error::validation(
            format!("unknown type {type_name}"),
            Some("type"),
        )),
    }
}

async fn merge_item_fields(tx: &mut Tx<'_>, item: ItemId) -> Result<Vec<ValueWire>> {
    let entity = "items.item";
    let record = record_id(item);
    let defs = wicket_customfields::definitions_for(tx, entity).await?;
    let mut out = Vec::with_capacity(defs.len());
    for def in defs {
        let value = wicket_customfields::get(tx, entity, record, &def.key).await?;
        out.push(wire_from_def(&def, value.as_ref()));
    }
    Ok(out)
}

fn fields_envelope(fields: Vec<ValueWire>) -> Result<JsonValue> {
    Ok(serde_json::to_value(&ListBody {
        data: fields,
        next_cursor: None,
        has_more: false,
    })?)
}

fn field_type_token_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    let tokens = [
        FieldType::String,
        FieldType::Text,
        FieldType::Integer,
        FieldType::Decimal,
        FieldType::Bool,
        FieldType::Date,
        FieldType::Enum,
        FieldType::Reference,
    ]
    .into_iter()
    .map(|ty| serde_json::Value::from(ty.as_str()))
    .collect();
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::String.into()),
        enum_values: Some(tokens),
        ..Default::default()
    }
    .into()
}

fn retired_status_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::String.into()),
        enum_values: Some(vec![serde_json::Value::from(
            DefinitionStatus::Retired.as_str(),
        )]),
        ..Default::default()
    }
    .into()
}

/// POST `/api/v1/customfields/definitions` body.
///
/// `type` is [`FieldType::as_str`] (`"string"`). `id`, when present, is rejected.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DefineBody {
    entity: String,
    key: String,
    #[serde(rename = "type")]
    #[schemars(schema_with = "field_type_token_schema")]
    field_type: String,
    label: String,
    #[serde(default)]
    validation_rule: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    indexed: bool,
    owner_module: String,
    #[serde(default)]
    id: Option<String>,
}

/// POST `/api/v1/customfields/definitions` 201 body. Subset of [`Definition`].
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DefineResponse {
    #[schemars(with = "uuid::Uuid")]
    id: String,
    entity: String,
    key: String,
}

/// POST /api/v1/customfields/definitions
pub async fn define(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match define_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn define_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, JsonValue)> {
    let body: DefineBody = parse_json(raw)?;
    if body.id.is_some() {
        return Err(Error::validation(
            "clients must not mint identifiers",
            Some("id"),
        ));
    }
    if body.entity.trim().is_empty() {
        return Err(Error::validation("entity is required", Some("entity")));
    }
    if body.key.trim().is_empty() {
        return Err(Error::validation("key is required", Some("key")));
    }
    let session =
        extract::require_mutation(state, headers, request_id, "customfields.define").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let spec = DefinitionSpec {
        entity: body.entity,
        key: body.key,
        field_type: parse_field_type(&body.field_type)?,
        label: body.label,
        validation_rule: body.validation_rule,
        required: body.required,
        indexed: body.indexed,
        owner_module: body.owner_module,
    };
    let ctx = write_context(
        &session,
        "customfields.define",
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
    let entity = spec.entity.clone();
    let field_key = spec.key.clone();
    let id = wicket_customfields::define(&mut tx, spec).await?;
    let payload = serde_json::to_value(&DefineResponse {
        id: id.as_uuid().to_string(),
        entity,
        key: field_key,
    })?;
    idempotency::remember(&mut tx, key, &hash, 201, &payload).await?;
    tx.commit().await?;
    Ok((201, payload))
}

/// Query for `GET /api/v1/customfields/definitions` (`entity` required).
#[derive(Debug, Deserialize)]
pub struct DefinitionsQuery {
    #[serde(default)]
    entity: Option<String>,
}

/// GET /api/v1/customfields/definitions?entity=
pub async fn definitions_for(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<DefinitionsQuery>,
) -> Response {
    let request_id = rid(&headers);
    match definitions_for_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn definitions_for_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: DefinitionsQuery,
) -> Result<JsonValue> {
    let entity = q
        .entity
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::validation("entity is required", Some("entity")))?;
    let session =
        extract::require_permission(state, headers, request_id, "customfields.view").await?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "customfields.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let defs = wicket_customfields::definitions_for(&mut tx, entity).await;
    tx.rollback().await?;
    let defs = defs?;
    Ok(serde_json::to_value(&ListBody {
        data: defs,
        next_cursor: None,
        has_more: false,
    })?)
}

/// POST /api/v1/customfields/definitions/{id}/retire
pub async fn retire(
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
        retire_inner(&state2, &headers2, &rid2, &id, &raw).await
    }) {
        Ok((st, v)) => json_status(st, v),
        Err(r) => r,
    }
}

async fn retire_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, JsonValue)> {
    let session =
        extract::require_mutation(state, headers, request_id, "customfields.retire").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let _expected = require_if_match(headers)?;
    let def_id = parse_uuid(id, "id", definition_id)?;
    let profile = state.kernel().profile.id.as_str();
    let engine = definition_engine(profile)?;
    let ctx = write_context(
        &session,
        "pending",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let ctx = wicket_customfields::retire_context(ctx, def_id);
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some(replay) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(replay);
    }
    wicket_customfields::retire(&mut tx, &engine, def_id, &ctx).await?;
    let payload = serde_json::to_value(&RetireResponse {
        id: def_id.as_uuid().to_string(),
        status: DefinitionStatus::Retired.as_str().to_owned(),
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &payload).await?;
    tx.commit().await?;
    Ok((200, payload))
}

/// POST `/api/v1/customfields/definitions/{id}/retire` body.
///
/// `status` is [`DefinitionStatus::as_str`] (`"retired"`), not the enum serde
/// name `"Retired"`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RetireResponse {
    #[schemars(with = "uuid::Uuid")]
    id: String,
    #[schemars(schema_with = "retired_status_schema")]
    status: String,
}

/// One field in [`SetBody`]. `type` is [`FieldType::as_str`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FieldWrite {
    key: String,
    #[serde(rename = "type")]
    #[schemars(schema_with = "field_type_token_schema")]
    field_type: String,
    value: JsonValue,
}

/// PUT `/api/v1/items/{id}/custom-fields` body.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetBody {
    fields: Vec<FieldWrite>,
}

/// PUT /api/v1/items/{id}/custom-fields
pub async fn set(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match set_inner(&state, &headers, &request_id, &id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn set_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<(u16, JsonValue)> {
    let body: SetBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "customfields.set").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let item_id = parse_uuid(id, "id", ItemId::from_uuid)?;
    wicket_mod_items::get(state.pool(), item_id).await?;
    let ctx = write_context(
        &session,
        "customfields.set",
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
    let record = record_id(item_id);
    for field in body.fields {
        if field.key.trim().is_empty() {
            return Err(Error::validation("key is required", Some("key")));
        }
        let value = parse_typed_value(&field.field_type, &field.value)?;
        wicket_customfields::set(&mut tx, "items.item", record, &field.key, value).await?;
    }
    let fields = merge_item_fields(&mut tx, item_id).await?;
    let payload = fields_envelope(fields)?;
    idempotency::remember(&mut tx, key, &hash, 200, &payload).await?;
    tx.commit().await?;
    Ok((200, payload))
}

/// GET /api/v1/items/{id}/custom-fields
pub async fn get_item_fields(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_item_fields_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_item_fields_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<JsonValue> {
    let session =
        extract::require_permission(state, headers, request_id, "customfields.view").await?;
    let item_id = parse_uuid(id, "id", ItemId::from_uuid)?;
    wicket_mod_items::get(state.pool(), item_id).await?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "customfields.view",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let fields = merge_item_fields(&mut tx, item_id).await;
    tx.rollback().await?;
    fields_envelope(fields?)
}

#[cfg(test)]
mod wire_freeze {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_type_tokens_match_value_wire() {
        let body = serde_json::to_value(schemars::schema_for!(DefineBody)).expect("define");
        let write = serde_json::to_value(schemars::schema_for!(FieldWrite)).expect("write");
        let wire = serde_json::to_value(schemars::schema_for!(ValueWire)).expect("wire");
        assert_eq!(
            body["properties"]["type"]["enum"],
            wire["properties"]["type"]["enum"]
        );
        assert_eq!(
            write["properties"]["type"]["enum"],
            wire["properties"]["type"]["enum"]
        );
        assert_eq!(body["properties"]["type"]["enum"][0], "string");
    }

    #[test]
    fn define_response_matches_legacy_blob() {
        let id = "01234567-89ab-4def-8123-456789abcdef";
        let entity = "items.item";
        let key = "shop_note";
        let blob = json!({
            "id": id,
            "entity": entity,
            "key": key,
        });
        let typed = serde_json::to_value(&DefineResponse {
            id: id.to_owned(),
            entity: entity.to_owned(),
            key: key.to_owned(),
        })
        .expect("json");
        assert_eq!(blob, typed);
    }

    #[test]
    fn retire_response_matches_legacy_blob() {
        let id = "01234567-89ab-4def-8123-456789abcdef";
        let blob = json!({
            "id": id,
            "status": "retired",
        });
        let typed = serde_json::to_value(&RetireResponse {
            id: id.to_owned(),
            status: DefinitionStatus::Retired.as_str().to_owned(),
        })
        .expect("json");
        assert_eq!(blob, typed);
        assert_ne!(
            serde_json::to_value(DefinitionStatus::Retired).expect("json"),
            json!("retired")
        );
    }

    #[test]
    fn fields_envelope_matches_legacy_blob() {
        let fields = vec![ValueWire {
            key: "shop_note".into(),
            type_name: "string".into(),
            value: json!("hello"),
            definition_version: 1,
        }];
        let blob = json!({
            "data": fields,
            "next_cursor": null,
            "has_more": false,
        });
        let typed = fields_envelope(fields).expect("json");
        assert_eq!(blob, typed);
    }

    #[test]
    fn definitions_envelope_matches_legacy_blob() {
        let defs: Vec<Definition> = Vec::new();
        let blob = json!({
            "data": defs,
            "next_cursor": null,
            "has_more": false,
        });
        let typed = serde_json::to_value(&ListBody {
            data: defs,
            next_cursor: None,
            has_more: false,
        })
        .expect("json");
        assert_eq!(blob, typed);
    }
}
