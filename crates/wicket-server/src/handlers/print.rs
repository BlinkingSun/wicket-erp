// w3b:print

//! HTTP transport for `wicket-print`. Published crate APIs only (R-2s-3).

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wicket_core::{Identifier, RecordRef};
use wicket_db::{ReadPool, Tx};
use wicket_documents::{BlobHash, BlobStore};
use wicket_esign::Manifestation;
use wicket_print::{
    Format, TemplateId, archive as print_archive, list_templates_on, log as print_log,
    manifestation_block, render as print_render,
};

use super::{H, json_status, parse_json, rid};
use crate::boot::AppState;
use crate::envelope::{ListBody, error_response};
use crate::error::{Error, Result};
use crate::extract;
use crate::idempotency;
use crate::session;
use crate::wire::parse_uuid;

use axum::body::Bytes;

/// Map `wicket_print::Error` into the docs/10 envelope.
pub fn envelope_arm(
    e: &wicket_print::Error,
) -> (&'static str, axum::http::StatusCode, Option<&str>, String) {
    match e {
        wicket_print::Error::NotFound => (
            "NOT_FOUND",
            axum::http::StatusCode::NOT_FOUND,
            None,
            e.to_string(),
        ),
        wicket_print::Error::UnknownTemplate(_) => (
            "NOT_FOUND",
            axum::http::StatusCode::NOT_FOUND,
            Some("template_id"),
            e.to_string(),
        ),
        wicket_print::Error::UnsupportedFormat => (
            "VALIDATION",
            axum::http::StatusCode::BAD_REQUEST,
            Some("format"),
            e.to_string(),
        ),
        wicket_print::Error::UnknownProfile(_) => (
            "VALIDATION",
            axum::http::StatusCode::BAD_REQUEST,
            Some("profile"),
            e.to_string(),
        ),
        wicket_print::Error::Documents(wicket_documents::Error::NotFound) => (
            "NOT_FOUND",
            axum::http::StatusCode::NOT_FOUND,
            None,
            e.to_string(),
        ),
        // Print has no StateMachine variant; keep INTERNAL for the rest.
        _ => (
            "INTERNAL",
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            None,
            e.to_string(),
        ),
    }
}

/// GET /api/v1/print/templates
pub async fn list_templates(State(state): State<AppState>, headers: H) -> Response {
    let request_id = rid(&headers);
    match list_templates_inner(&state, &headers, &request_id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_response(e, &request_id),
    }
}

async fn list_templates_inner(
    state: &AppState,
    headers: &H,
    request_id: &str,
) -> Result<ListBody<wicket_print::TemplateSummary>> {
    let _session =
        extract::require_permission(state, headers, request_id, "print.templates").await?;
    let rows = list_templates_on(
        &ReadPool::new(state.pool().clone()),
        state.kernel().profile.id.as_str(),
    )
    .await?;
    Ok(ListBody {
        data: rows,
        next_cursor: None,
        has_more: false,
    })
}

/// POST /api/v1/print/render
pub async fn render(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match render_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => json_status(st, v),
        Err(e) => error_response(e, &request_id),
    }
}

/// Record locator shared by print render and archive request bodies.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordBody {
    table: String,
    id: String,
    version: i64,
}

/// POST `/api/v1/print/render` body.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderBody {
    record: RecordBody,
    format: String,
    template_id: String,
}

/// POST `/api/v1/print/archive` body.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArchiveBody {
    record: RecordBody,
    output_hash: String,
}

/// POST `/api/v1/print/render` success body.
///
/// This is the archival blob the handler emits today, not [`wicket_print::Rendered`].
/// `output_hash` and `blob` hashes elsewhere are lowercase hex. `bytes_base64` is the
/// rendition bytes. `manifestation` is [`Manifestation`] as `wicket-esign` serializes
/// it. That type lives in a sibling crate, so its schema is hand-specified here rather
/// than derived on a copy.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RenderPrintResponse {
    /// Lowercase hex SHA-256 of the rendition bytes.
    output_hash: String,
    /// Template row version used.
    template_version: i32,
    /// Crate version that produced the bytes.
    renderer_version: String,
    /// Base64 of the rendition bytes.
    bytes_base64: String,
    /// Esign snapshots for the record. Empty when none exist.
    #[schemars(schema_with = "manifestation_list_schema")]
    manifestation: Vec<Manifestation>,
}

/// POST `/api/v1/print/archive` success body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ArchivePrintResponse {
    /// Lowercase hex SHA-256 of the archived blob.
    blob_hash: String,
}

/// Schema for `Vec<Manifestation>` matching that type's serde shape.
///
/// Keys are always present. `reason` and `superseded_by_version` are null when unset.
fn manifestation_list_schema(generator: &mut SchemaGenerator) -> Schema {
    let signature = object(
        vec![
            ("id", generator.subschema_for::<String>()),
            ("signer_id", generator.subschema_for::<String>()),
            ("printed_name", generator.subschema_for::<String>()),
            ("meaning", generator.subschema_for::<String>()),
            ("reason", generator.subschema_for::<Option<String>>()),
            ("signed_at", generator.subschema_for::<String>()),
            ("signed_at_zone", generator.subschema_for::<String>()),
            ("signed_at_local", generator.subschema_for::<String>()),
            (
                "record",
                object(
                    vec![
                        ("table", generator.subschema_for::<String>()),
                        ("doc_type", generator.subschema_for::<String>()),
                        ("id", generator.subschema_for::<String>()),
                        ("version", generator.subschema_for::<i64>()),
                    ],
                    &["table", "doc_type", "id", "version"],
                ),
            ),
            ("record_content_hash", generator.subschema_for::<String>()),
            ("credential_kind", generator.subschema_for::<String>()),
            ("components_used", generator.subschema_for::<Vec<String>>()),
            ("superseded", generator.subschema_for::<bool>()),
            (
                "superseded_by_version",
                with_default_null(generator.subschema_for::<Option<i64>>()),
            ),
        ],
        &[
            "id",
            "signer_id",
            "printed_name",
            "meaning",
            "reason",
            "signed_at",
            "signed_at_zone",
            "signed_at_local",
            "record",
            "record_content_hash",
            "credential_kind",
            "components_used",
            "superseded",
            "superseded_by_version",
        ],
    );
    array_of(object(vec![("signature", signature)], &["signature"]))
}

fn with_default_null(schema: Schema) -> Schema {
    let mut obj = schema.into_object();
    obj.metadata().default = Some(Value::Null);
    Schema::Object(obj)
}

fn object(fields: Vec<(&str, Schema)>, required: &[&str]) -> Schema {
    let mut obj = SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        ..Default::default()
    };
    {
        let validation = obj.object();
        for (name, schema) in fields {
            validation.properties.insert(name.to_owned(), schema);
        }
        for name in required {
            validation.required.insert((*name).to_owned());
        }
    }
    Schema::Object(obj)
}

fn array_of(items: Schema) -> Schema {
    let mut obj = SchemaObject {
        instance_type: Some(InstanceType::Array.into()),
        ..Default::default()
    };
    obj.array().items = Some(items.into());
    Schema::Object(obj)
}

fn parse_record(body: &RecordBody) -> Result<RecordRef> {
    let id = parse_uuid(&body.id, "record.id", Identifier::from_uuid)?;
    if body.table.is_empty() {
        return Err(Error::validation(
            "record.table is required",
            Some("record.table"),
        ));
    }
    Ok(RecordRef {
        table: body.table.clone(),
        id,
        version: body.version,
    })
}

fn hex_hash(bytes: &[u8; 32]) -> String {
    wicket_audit::sha256::hex(bytes)
}

fn parse_hex32(s: &str, field: &str) -> Result<[u8; 32]> {
    if s.len() != 64 {
        return Err(Error::validation(
            "hash must be 64 lowercase hex characters",
            Some(field),
        ));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let hi = hex_nibble(chunk[0], field)?;
        let lo = hex_nibble(chunk[1], field)?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_nibble(b: u8, field: &str) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(Error::validation(
            "hash must be 64 lowercase hex characters",
            Some(field),
        )),
    }
}

fn b64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let remaining = input.len() - i;
        let b0 = input[i];
        let b1 = if remaining > 1 { input[i + 1] } else { 0 };
        let b2 = if remaining > 2 { input[i + 2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if remaining > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if remaining > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

async fn render_inner(
    state: &AppState,
    headers: &HeaderMap,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: RenderBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "print.render").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let record = parse_record(&body.record)?;
    let format = Format::parse(&body.format)
        .ok_or_else(|| Error::validation("format must be html or pdf", Some("format")))?;
    if body.template_id == "item_label" {
        return Err(Error::validation(
            "item_label is ADR 0009 territory and is not mounted",
            Some("template_id"),
        ));
    }
    let template = TemplateId::new(&body.template_id);
    let ctx = session::write_context(
        &session,
        "print.render",
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
    let rendered = print_render(&mut tx, record.clone(), format, template).await?;
    let manifestation = manifestation_block(&mut tx, record).await?;
    let payload = serde_json::to_value(RenderPrintResponse {
        output_hash: hex_hash(&rendered.output_hash),
        template_version: rendered.template_version,
        renderer_version: rendered.renderer_version,
        bytes_base64: b64_encode(&rendered.bytes),
        manifestation,
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &payload).await?;
    tx.commit().await?;
    Ok((200, payload))
}

/// POST /api/v1/print/archive
pub async fn archive(State(state): State<AppState>, headers: H, body: Bytes) -> Response {
    let request_id = rid(&headers);
    match archive_inner(&state, &headers, &request_id, &body).await {
        Ok((st, v)) => {
            state.blobs().keep_puts();
            json_status(st, v)
        }
        Err(e) => {
            state.blobs().discard_uncommitted();
            error_response(e, &request_id)
        }
    }
}

async fn archive_inner(
    state: &AppState,
    headers: &HeaderMap,
    request_id: &str,
    raw: &[u8],
) -> Result<(u16, Value)> {
    let body: ArchiveBody = parse_json(raw)?;
    let session = extract::require_mutation(state, headers, request_id, "print.archive").await?;
    let key = idempotency::require_key(headers)?;
    let hash = idempotency::body_hash(raw);
    let record = parse_record(&body.record)?;
    let output_hash = parse_hex32(&body.output_hash, "output_hash")?;
    let ctx = session::write_context(
        &session,
        "print.archive",
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
    let rows = print_log(&mut tx, record.clone()).await?;
    let row = rows
        .into_iter()
        .find(|r| r.output_hash == output_hash)
        .ok_or_else(|| Error::not_found("no render_log row for this output_hash"))?;
    let format = Format::parse(&row.output_format).ok_or(wicket_print::Error::UnsupportedFormat)?;
    let rendered = print_render(
        &mut tx,
        record.clone(),
        format,
        TemplateId::new(&row.template_id),
    )
    .await?;
    if rendered.output_hash != output_hash {
        tx.rollback().await?;
        return Err(Error::conflict(
            "re-render did not reproduce the requested output_hash",
            Some("output_hash"),
        ));
    }
    let blob: BlobHash = print_archive(&mut tx, &rendered, record, state.blobs()).await?;
    let payload = serde_json::to_value(ArchivePrintResponse {
        blob_hash: blob.to_hex(),
    })?;
    idempotency::remember(&mut tx, key, &hash, 200, &payload).await?;
    tx.commit().await?;
    Ok((200, payload))
}

#[cfg(test)]
mod wire {
    use super::{ArchivePrintResponse, RenderPrintResponse};
    use serde_json::{Value, json};
    use wicket_esign::{ManifestRecord, Manifestation, SignatureManifest};

    fn sample_manifestation() -> Manifestation {
        Manifestation {
            signature: SignatureManifest {
                id: "sig-1".into(),
                signer_id: "principal-1".into(),
                printed_name: "M. Reyes".into(),
                meaning: "Approved".into(),
                reason: None,
                signed_at: "2026-09-19T00:00:00Z".into(),
                signed_at_zone: "America/New_York".into(),
                signed_at_local: "2026-09-18T20:00:00-04:00".into(),
                record: ManifestRecord {
                    table: "generic.record".into(),
                    doc_type: "generic".into(),
                    id: "11111111-1111-1111-1111-111111111111".into(),
                    version: 1,
                },
                record_content_hash: "ab".repeat(32),
                credential_kind: "password".into(),
                components_used: vec!["identification".into()],
                superseded: false,
                superseded_by_version: None,
            },
        }
    }

    fn render_value(manifestation: Vec<Manifestation>) -> Value {
        serde_json::to_value(RenderPrintResponse {
            output_hash: "aa".repeat(32),
            template_version: 1,
            renderer_version: "0.1.0".into(),
            bytes_base64: "QQ==".into(),
            manifestation,
        })
        .expect("render json")
    }

    #[test]
    fn render_named_type_matches_legacy_blob() {
        let manifestation = vec![sample_manifestation()];
        let blob = json!({
            "output_hash": "aa".repeat(32),
            "template_version": 1,
            "renderer_version": "0.1.0",
            "bytes_base64": "QQ==",
            "manifestation": manifestation.clone(),
        });
        assert_eq!(blob, render_value(manifestation));
    }

    #[test]
    fn archive_named_type_matches_legacy_blob() {
        let blob = json!({ "blob_hash": "bb".repeat(32) });
        let typed = serde_json::to_value(ArchivePrintResponse {
            blob_hash: "bb".repeat(32),
        })
        .expect("archive json");
        assert_eq!(blob, typed);
    }

    #[test]
    fn render_omits_no_fields_when_manifestation_empty() {
        let blob = json!({
            "output_hash": "cc".repeat(32),
            "template_version": 2,
            "renderer_version": "0.1.0",
            "bytes_base64": "",
            "manifestation": [],
        });
        let typed = serde_json::to_value(RenderPrintResponse {
            output_hash: "cc".repeat(32),
            template_version: 2,
            renderer_version: "0.1.0".into(),
            bytes_base64: String::new(),
            manifestation: Vec::new(),
        })
        .expect("empty manifestation json");
        assert_eq!(blob, typed);
        assert!(matches!(typed["manifestation"], Value::Array(ref a) if a.is_empty()));
    }

    fn type_includes(schema: &Value, expected: &str) -> bool {
        match &schema["type"] {
            Value::String(s) => s == expected,
            Value::Array(items) => items.iter().any(|v| v.as_str() == Some(expected)),
            _ => false,
        }
    }

    fn assert_schema_covers(sample: &Value, schema: &Value) {
        match sample {
            Value::Object(map) => {
                assert!(type_includes(schema, "object"), "{schema}");
                let props = schema["properties"].as_object().expect("properties");
                let mut sample_keys: Vec<_> = map.keys().cloned().collect();
                let mut schema_keys: Vec<_> = props.keys().cloned().collect();
                sample_keys.sort();
                schema_keys.sort();
                assert_eq!(sample_keys, schema_keys, "schema keys");
                let mut required: Vec<_> = schema["required"]
                    .as_array()
                    .expect("required")
                    .iter()
                    .map(|v| v.as_str().expect("required name").to_owned())
                    .collect();
                required.sort();
                assert_eq!(required, sample_keys, "always-present keys");
                for (key, value) in map {
                    assert_schema_covers(value, &props[key]);
                }
            }
            Value::Array(items) => {
                assert!(type_includes(schema, "array"), "{schema}");
                if let Some(first) = items.first() {
                    assert_schema_covers(first, &schema["items"]);
                }
            }
            Value::Null => assert!(type_includes(schema, "null"), "{schema}"),
            Value::Bool(_) => assert!(type_includes(schema, "boolean"), "{schema}"),
            Value::Number(_) => assert!(
                type_includes(schema, "integer") || type_includes(schema, "number"),
                "{schema}"
            ),
            Value::String(_) => assert!(type_includes(schema, "string"), "{schema}"),
        }
    }

    #[test]
    fn render_manifestation_schema_matches_wire() {
        let nulls = render_value(vec![sample_manifestation()]);
        assert!(nulls["manifestation"][0]["signature"]["reason"].is_null());
        assert!(nulls["manifestation"][0]["signature"]["superseded_by_version"].is_null());
        let mut valued = sample_manifestation();
        valued.signature.reason = Some("because".into());
        valued.signature.superseded_by_version = Some(4);
        let values = render_value(vec![valued]);
        let root =
            serde_json::to_value(schemars::schema_for!(RenderPrintResponse)).expect("schema");
        assert_schema_covers(&nulls, &root);
        assert_schema_covers(&values, &root);
    }
}
