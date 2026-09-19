//! Kernel HTTP handlers: health, OpenAPI, validation manifest, audit export,
//! login, logout, navigation.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use wicket_db::Tx;
use wicket_identity::{PasswordProvider, Provider};

use super::{H, parse_json, rid};
use crate::boot::AppState;
use crate::envelope::error_response;
use crate::error::{Error, Result};
use crate::extract;
use crate::idempotency;
use crate::session::{self, write_context};

use axum::body::Bytes;

pub async fn login(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match login_inner(&state, &headers, &request_id, &body).await {
        Ok(r) => r,
        Err(e) => error_response(e, &request_id),
    }
}

/// POST `/api/v1/identity/login` body.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoginBody {
    username: String,
    password: String,
}

/// POST `/api/v1/identity/login` success body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LoginResponse {
    session_id: String,
    principal_id: String,
    display_name: String,
    csrf: String,
}

/// GET `/api/v1/navigation` body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct NavigationBody {
    visible: Vec<String>,
    hidden: Vec<String>,
}

async fn login_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
    raw: &[u8],
) -> Result<Response> {
    let body: LoginBody = parse_json(raw)?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let write = state.write_pool();
    let mut ctx = session::system_ctx("identity.login");
    ctx.request_id = Some(request_id.to_string());
    ctx.config_version = Some(state.kernel().profile.spec_version.clone());
    let mut tx = Tx::begin(&write, &ctx).await?;
    if let Some((status, stored)) = idempotency::replay(&mut tx, key, &hash).await? {
        tx.commit().await?;
        return login_cookies(headers, status, stored);
    }
    let sess = PasswordProvider
        .login(&mut tx, &body.username, &body.password, None, ip)
        .await?;
    let principal = wicket_identity::load_principal(state.pool(), sess.principal).await?;
    let csrf = Uuid::now_v7().to_string();
    let actor = wicket_core::Actor {
        id: sess.principal.0,
        kind: wicket_core::ActorKind::User,
    };
    let permissions = session::granted_permissions(&mut tx, actor).await?;
    session::store(
        &mut tx,
        sess.id,
        sess.principal,
        &principal.display_name,
        &csrf,
        sess.expires_at,
        &permissions,
    )
    .await?;
    let payload = serde_json::to_value(&LoginResponse {
        session_id: sess.id.to_string(),
        principal_id: sess.principal.as_uuid().to_string(),
        display_name: principal.display_name,
        csrf,
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &payload).await?;
    tx.commit().await?;
    login_cookies(headers, 200, payload)
}

fn login_cookies(headers: &H, status: u16, payload: Value) -> Result<Response> {
    let session_id = payload
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Config("login replay missing session_id".into()))?
        .to_string();
    let csrf = payload
        .get("csrf")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Config("login replay missing csrf".into()))?
        .to_string();
    let mut resp = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(payload),
    )
        .into_response();
    let secure = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|p| p.eq_ignore_ascii_case("https"));
    set_cookie(
        resp.headers_mut(),
        "wicket_session",
        &session_id,
        true,
        secure,
    );
    set_cookie(resp.headers_mut(), "wicket_csrf", &csrf, false, secure);
    Ok(resp)
}

fn set_cookie(headers: &mut H, name: &str, value: &str, http_only: bool, secure: bool) {
    let mut c = format!("{name}={value}; Path=/; SameSite=Lax");
    if http_only {
        c.push_str("; HttpOnly");
    }
    if secure {
        c.push_str("; Secure");
    }
    if let Ok(v) = HeaderValue::from_str(&c) {
        headers.append(axum::http::header::SET_COOKIE, v);
    }
}

/// POST /api/v1/identity/logout
pub async fn logout(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match logout_inner(&state, &headers, &request_id, &body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn logout_inner(state: &AppState, headers: &H, request_id: &str, raw: &[u8]) -> Result<()> {
    let session = extract::require_mutation(state, headers, request_id, "identity.session").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let write = state.write_pool();
    let ctx = write_context(
        &session,
        "identity.logout",
        request_id,
        headers,
        &state.kernel().profile.spec_version,
    );
    let mut tx = Tx::begin(&write, &ctx).await?;
    if idempotency::replay(&mut tx, key, &hash).await?.is_some() {
        tx.commit().await?;
        return Ok(());
    }
    session::drop_session(&mut tx, session.id).await?;
    let empty = json!({});
    idempotency::remember(&mut tx, key, &hash, 204, &empty).await?;
    tx.commit().await?;
    Ok(())
}

/// GET /api/v1/openapi.json — unauthenticated.
pub async fn openapi(State(state): State<AppState>) -> Json<Value> {
    Json(crate::openapi::document(&state))
}

/// GET /api/v1/iq/manifest
pub async fn manifest(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    if let Err(e) =
        extract::require_permission(&state, &headers, &request_id, "validation.manifest.read").await
    {
        return error_response(e, &request_id);
    }
    match crate::boot::live_manifest(state.pool()).await {
        Ok(m) => Json(json!({
            "profile_id": m.profile_id,
            "spec_version": m.spec_version,
            "content_hash": m.content_hash,
            "modules": m.modules,
            "signature_edges": m.signature_edges,
            "kernel_order": m.kernel_order,
        }))
        .into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

/// GET /api/v1/navigation
pub async fn navigation(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    if let Err(e) =
        extract::require_permission(&state, &headers, &request_id, "identity.session").await
    {
        return error_response(e, &request_id);
    }
    Json(NavigationBody {
        visible: state.kernel().profile.navigation.visible.clone(),
        hidden: state.kernel().profile.navigation.hidden.clone(),
    })
    .into_response()
}

/// GET /api/v1/audit — admin SELECT (permission-gated; kernel, not a module).
pub async fn audit_export(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    if let Err(e) = extract::require_permission(&state, &headers, &request_id, "audit.export").await
    {
        return error_response(e, &request_id);
    }
    match wicket_audit::head(state.pool()).await {
        Ok(head) => Json(json!({
            "head": head.map(|h| json!({
                "seq": h.seq,
                "xid": h.xid,
                "row_count": h.row_count,
                "chain_algo": h.chain_algo,
            })),
        }))
        .into_response(),
        Err(e) => error_response(e.into(), &request_id),
    }
}

pub async fn health() -> &'static str {
    crate::version()
}
