//! Esign and calibration HTTP body types.
//!
//! Handler functions stay in `mod.rs` because `tests/esign.rs` include_str-pins
//! `esign_mint_inner` and the manifestation call sites to that file.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// POST `/api/v1/calibration/certificates/{id}/approve` success body.
///
/// The engine emits `{ id, status: "approved" }`. `status` is that literal,
/// not an open string of future states.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalibrationApprovedBody {
    pub(super) id: String,
    /// Always the string `"approved"`.
    #[schemars(schema_with = "approved_status_schema")]
    pub(super) status: String,
}

fn approved_status_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::SingleOrVec::Single(Box::new(
            schemars::schema::InstanceType::String,
        ))),
        enum_values: Some(vec![serde_json::json!("approved")]),
        ..Default::default()
    }
    .into()
}

/// POST `/api/v1/esign/signatures` request body.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EsignMintBody {
    pub(super) meaning: String,
    #[serde(default)]
    pub(super) reason: Option<String>,
    pub(super) record: EsignRecordBody,
    pub(super) identification: EsignIdentBody,
    #[serde(default)]
    pub(super) doc_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct EsignRecordBody {
    pub(super) table: String,
    pub(super) id: String,
    pub(super) version: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct EsignIdentBody {
    #[serde(default)]
    pub(super) code: Option<String>,
    #[serde(default)]
    pub(super) secret: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn calibration_approved_body_matches_frozen_blob() {
        let id = "01900000-0000-7000-8000-000000000001";
        let named = serde_json::to_value(CalibrationApprovedBody {
            id: id.into(),
            status: "approved".into(),
        })
        .expect("json");
        assert_eq!(named, json!({"id": id, "status": "approved"}));
    }

    #[test]
    fn calibration_status_schema_is_the_literal_approved() {
        let root =
            serde_json::to_value(schemars::schema_for!(CalibrationApprovedBody)).expect("schema");
        let status = &root["properties"]["status"];
        assert_eq!(status["type"], "string");
        assert_eq!(status["enum"], json!(["approved"]));
    }

    #[test]
    fn mint_body_parses_the_handler_shape_and_ignores_unknown_fields() {
        let raw = json!({
            "meaning": "Approved",
            "record": {
                "table": "sm.instance",
                "id": "01900000-0000-7000-8000-000000000001",
                "version": 1
            },
            "identification": { "code": "MREYES", "secret": "x" },
            "extra": true
        });
        let body: EsignMintBody = serde_json::from_value(raw).expect("parse");
        assert_eq!(body.meaning, "Approved");
        assert!(body.reason.is_none());
        assert!(body.doc_type.is_none());
        assert_eq!(body.record.table, "sm.instance");
        assert_eq!(body.record.version, 1);
        assert_eq!(body.identification.code.as_deref(), Some("MREYES"));
        assert_eq!(body.identification.secret.as_deref(), Some("x"));

        let omitted: EsignMintBody = serde_json::from_value(json!({
            "meaning": "Approved",
            "record": { "table": "t", "id": "x", "version": 2 },
            "identification": {}
        }))
        .expect("defaults");
        assert!(omitted.identification.code.is_none());
        assert!(omitted.identification.secret.is_none());
        assert_eq!(omitted.record.version, 2);
    }
}
