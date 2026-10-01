// w3a:identity

//! HTTP adapters for existing `wicket_identity` capabilities (T-34 / W3a).
//!
//! No SQL. The two-person reset pair is not mounted. Signing rotation is
//! bound to `session.principal` only.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wicket_core::Identifier;
use wicket_db::Tx;
use wicket_identity::{Principal, PrincipalKind, PrincipalStatus, Role, UserId};

use super::{H, json_status, nonempty, parse_json, parse_limit, rid};
use crate::boot::AppState;
use crate::envelope::{ListBody, error_response};
use crate::error::{Error, Result};
use crate::extract;
use crate::idempotency;
use crate::session::write_context;
use crate::wire::parse_uuid;

use axum::body::Bytes;

fn user_id(u: uuid::Uuid) -> UserId {
    UserId::from_identifier(Identifier::from_uuid(u))
}

fn map_identity(e: wicket_identity::Error) -> Error {
    match e {
        wicket_identity::Error::UsernameReused => {
            Error::conflict("username is never reused", Some("username"))
        }
        wicket_identity::Error::InvalidLimit => {
            Error::validation("limit must be between 1 and 200", Some("limit"))
        }
        other => other.into(),
    }
}

fn require_nonempty<'a>(value: &'a str, field: &'static str) -> Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::validation(
            format!("{field} is required"),
            Some(field),
        ));
    }
    Ok(trimmed)
}

/// GET `/api/v1/identity/me` (and principal read) body. Timestamps stay RFC 3339
/// strings at seconds precision, matching the previous `json!` emission.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PrincipalBody {
    id: String,
    principal_kind: String,
    username: String,
    display_name: String,
    status: String,
    created_at: String,
    deactivated_at: Option<String>,
}

fn principal_wire(p: &Principal) -> PrincipalBody {
    PrincipalBody {
        id: p.id.as_uuid().to_string(),
        principal_kind: match p.principal_kind {
            PrincipalKind::User => "User".to_owned(),
            PrincipalKind::Service => "Service".to_owned(),
            PrincipalKind::Migration => "Migration".to_owned(),
        },
        username: p.username.clone(),
        display_name: p.display_name.clone(),
        status: match p.status {
            PrincipalStatus::Active => "Active".to_owned(),
            PrincipalStatus::Inactive => "Inactive".to_owned(),
        },
        created_at: p
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        deactivated_at: p
            .deactivated_at
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    }
}

fn principal_body(p: &Principal) -> Value {
    serde_json::to_value(principal_wire(p)).expect("PrincipalBody")
}

/// GET `/api/v1/identity/roles` (and role read) body.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RoleBody {
    id: String,
    name: String,
    permissions: Vec<String>,
}

fn role_wire(r: &Role) -> RoleBody {
    RoleBody {
        id: r.id.as_uuid().to_string(),
        name: r.name.clone(),
        permissions: r.permissions.clone(),
    }
}

fn optional_principal_kind_json_schema(
    _gen: &mut schemars::SchemaGenerator,
) -> schemars::schema::Schema {
    serde_json::from_value(json!({
        "anyOf": [
            {
                "type": "string",
                "enum": ["User", "Service", "Migration"]
            },
            { "type": "null" }
        ]
    }))
    .expect("optional principal_kind schema")
}

/// POST `/api/v1/identity/principals` body.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePrincipalBody {
    username: String,
    display_name: String,
    password: String,
    #[serde(default)]
    #[schemars(schema_with = "optional_principal_kind_json_schema")]
    principal_kind: Option<PrincipalKind>,
    #[serde(default)]
    id: Option<String>,
}

/// POST `/api/v1/identity/principals/{id}/rename` body.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameBody {
    display_name: String,
}

/// Login-credential mutation bodies (`password` field).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PasswordBody {
    password: String,
}

/// POST `/api/v1/identity/me/signing-credential` body.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SigningSecretBody {
    secret: String,
}

/// Empty JSON object accepted on deactivate (empty bytes also accepted).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyBody {}

/// Idempotency replay shape and OpenAPI stand-in for HTTP 204 success (no response body).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NoContentJson {}

/// POST /api/v1/identity/principals
pub async fn create_principal(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match create_principal_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

async fn create_principal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: CreatePrincipalBody = parse_json(raw)?;
    if body.id.is_some() {
        return Err(Error::validation(
            "clients must not mint identifiers",
            Some("id"),
        ));
    }
    let username = require_nonempty(&body.username, "username")?;
    let display_name = require_nonempty(&body.display_name, "display_name")?;
    let password = require_nonempty(&body.password, "password")?;
    let kind = body.principal_kind.unwrap_or(PrincipalKind::User);
    let session = extract::require_mutation(state, headers, request_id, "identity.manage").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.create",
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
    let principal = wicket_identity::create_principal(&mut tx, kind, username, display_name)
        .await
        .map_err(map_identity)?;
    wicket_identity::set_login_credential(&mut tx, principal.id, password)
        .await
        .map_err(map_identity)?;
    let payload = principal_body(&principal);
    idempotency::remember(&mut tx, key, &hash, 201, &payload).await?;
    tx.commit().await?;
    Ok((201, payload))
}

/// GET /api/v1/identity/principals/{id}
pub async fn get_principal(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match get_principal_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_principal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let _session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let id = parse_uuid(id, "id", user_id)?;
    let principal = wicket_identity::load_principal(state.pool(), id)
        .await
        .map_err(map_identity)?;
    Ok(principal_body(&principal))
}

/// POST /api/v1/identity/principals/{id}/rename
pub async fn rename_principal(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match rename_principal_inner(&state, &headers, &request_id, &id, &body).await {
        Ok(status) => StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn rename_principal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<u16> {
    let body: RenameBody = parse_json(raw)?;
    let display_name = require_nonempty(&body.display_name, "display_name")?;
    let id = parse_uuid(id, "id", user_id)?;
    let session = extract::require_mutation(state, headers, request_id, "identity.manage").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.rename",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, _)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(status);
    }
    wicket_identity::rename_principal(&mut tx, id, display_name)
        .await
        .map_err(map_identity)?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(204)
}

/// POST /api/v1/identity/principals/{id}/deactivate
pub async fn deactivate_principal(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match deactivate_principal_inner(&state, &headers, &request_id, &id, &body).await {
        Ok(status) => StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn deactivate_principal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<u16> {
    let _: EmptyBody = if raw.is_empty() {
        EmptyBody {}
    } else {
        parse_json(raw)?
    };
    let id = parse_uuid(id, "id", user_id)?;
    let session = extract::require_mutation(state, headers, request_id, "identity.manage").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.deactivate",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, _)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(status);
    }
    wicket_identity::deactivate_principal(&mut tx, id)
        .await
        .map_err(map_identity)?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(204)
}

/// POST /api/v1/identity/principals/{id}/login-credential
pub async fn reset_login_credential(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match reset_login_credential_inner(&state, &headers, &request_id, &id, &body).await {
        Ok(status) => StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn reset_login_credential_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
    raw: &[u8],
) -> Result<u16> {
    let body: PasswordBody = parse_json(raw)?;
    let password = require_nonempty(&body.password, "password")?;
    let id = parse_uuid(id, "id", user_id)?;
    let session = extract::require_mutation(state, headers, request_id, "identity.manage").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.reset_login",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, _)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(status);
    }
    wicket_identity::set_login_credential(&mut tx, id, password)
        .await
        .map_err(map_identity)?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(204)
}

/// GET /api/v1/identity/me
pub async fn get_own_profile(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    match get_own_profile_inner(&state, &headers, &request_id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_own_profile_inner(state: &AppState, headers: &H, request_id: &str) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.session").await?;
    let principal = wicket_identity::load_principal(state.pool(), session.principal)
        .await
        .map_err(map_identity)?;
    Ok(principal_body(&principal))
}

/// POST /api/v1/identity/me/login-credential
pub async fn change_own_login_credential(
    State(state): State<AppState>,
    headers: H,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match change_own_login_credential_inner(&state, &headers, &request_id, &body).await {
        Ok(status) => StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn change_own_login_credential_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<u16> {
    let body: PasswordBody = parse_json(raw)?;
    let password = require_nonempty(&body.password, "password")?;
    let session = extract::require_mutation(state, headers, request_id, "identity.session").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.change_login",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, _)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(status);
    }
    wicket_identity::set_login_credential(&mut tx, session.principal, password)
        .await
        .map_err(map_identity)?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(204)
}

/// POST /api/v1/identity/me/signing-credential
///
/// The only HTTP operation that writes `identity.signing_credential`. The
/// principal id is `session.principal`; path and body cannot name a target.
pub async fn set_own_signing_credential(
    State(state): State<AppState>,
    headers: H,
    body: Bytes,
) -> Response {
    let request_id = rid(&headers);
    match set_own_signing_credential_inner(&state, &headers, &request_id, &body).await {
        Ok(status) => StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn set_own_signing_credential_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<u16> {
    let body: SigningSecretBody = parse_json(raw)?;
    let secret = require_nonempty(&body.secret, "secret")?;
    let session = extract::require_mutation(state, headers, request_id, "identity.session").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ctx = write_context(
        &session,
        "identity.set_signing",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let write = state.write_pool();
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, _)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return Ok(status);
    }
    wicket_identity::set_signing_credential(&mut tx, session.principal, secret)
        .await
        .map_err(map_identity)?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(204)
}

#[derive(Debug, Default, Deserialize)]
pub struct PrincipalListQ {
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
}

/// GET /api/v1/identity/principals
pub async fn list_principals(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<PrincipalListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_principals_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_principals_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: PrincipalListQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let limit = parse_limit(nonempty(&q.limit))?.map(i64::from);
    let cursor = nonempty(&q.cursor);
    if let Some(c) = cursor {
        let _ = parse_uuid(c, "cursor", Identifier::from_uuid)?;
    }
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "identity.manage",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_identity::list_principals(&mut tx, limit, cursor, None, None).await;
    tx.rollback().await?;
    let page = page.map_err(map_identity)?;
    Ok(serde_json::to_value(&ListBody {
        data: page.data.iter().map(principal_wire).collect::<Vec<_>>(),
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    })?)
}

#[derive(Debug, Default, Deserialize)]
pub struct UsernameQ {
    #[serde(default)]
    username: Option<String>,
}

/// GET /api/v1/identity/principals/by-username
pub async fn get_principal_by_username(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<UsernameQ>,
) -> Response {
    let request_id = rid(&headers);
    match get_principal_by_username_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_principal_by_username_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: UsernameQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let username = nonempty(&q.username)
        .ok_or_else(|| Error::validation("username is required", Some("username")))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "identity.manage",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let principal = wicket_identity::load_principal_by_username(&mut tx, username).await;
    tx.rollback().await?;
    Ok(principal_body(&principal.map_err(map_identity)?))
}

#[derive(Debug, Default, Deserialize)]
pub struct RoleListQ {
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
}

/// GET /api/v1/identity/roles
pub async fn list_roles(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<RoleListQ>,
) -> Response {
    let request_id = rid(&headers);
    match list_roles_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_roles_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: RoleListQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let limit = parse_limit(nonempty(&q.limit))?.map(i64::from);
    let cursor = nonempty(&q.cursor);
    if let Some(c) = cursor {
        let _ = parse_uuid(c, "cursor", Identifier::from_uuid)?;
    }
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "identity.manage",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let page = wicket_identity::list_roles(&mut tx, limit, cursor).await;
    tx.rollback().await?;
    let page = page.map_err(map_identity)?;
    Ok(serde_json::to_value(&ListBody {
        data: page.data.iter().map(role_wire).collect::<Vec<_>>(),
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    })?)
}

#[derive(Debug, Default, Deserialize)]
pub struct RoleNameQ {
    #[serde(default)]
    name: Option<String>,
}

/// GET /api/v1/identity/roles/by-name
pub async fn get_role_by_name(
    State(state): State<AppState>,
    headers: H,
    Query(q): Query<RoleNameQ>,
) -> Response {
    let request_id = rid(&headers);
    match get_role_by_name_inner(&state, &headers, &request_id, q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn get_role_by_name_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    q: RoleNameQ,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let name =
        nonempty(&q.name).ok_or_else(|| Error::validation("name is required", Some("name")))?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "identity.manage",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let role = wicket_identity::load_role_by_name(&mut tx, name).await;
    tx.rollback().await?;
    Ok(serde_json::to_value(role_wire(
        &role.map_err(map_identity)?,
    ))?)
}

/// GET /api/v1/identity/principals/{id}/roles
pub async fn list_roles_for_principal(
    State(state): State<AppState>,
    headers: H,
    Path(id): Path<String>,
) -> Response {
    let request_id = rid(&headers);
    match list_roles_for_principal_inner(&state, &headers, &request_id, &id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_roles_for_principal_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    id: &str,
) -> Result<Value> {
    let session =
        extract::require_permission(state, headers, request_id, "identity.manage").await?;
    let id = parse_uuid(id, "id", user_id)?;
    let write = crate::read::pool(state);
    let mut tx = crate::read::begin(
        &write,
        &session,
        "identity.manage",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    )
    .await?;
    let roles = wicket_identity::list_roles_for_principal(&mut tx, id).await;
    tx.rollback().await?;
    let roles = roles.map_err(map_identity)?;
    Ok(serde_json::to_value(&ListBody {
        data: roles.iter().map(role_wire).collect::<Vec<_>>(),
        next_cursor: None,
        has_more: false,
    })?)
}
