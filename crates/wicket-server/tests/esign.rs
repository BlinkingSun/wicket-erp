//! HTTP esign routes (D-2b-8) under both installation profiles.

#![allow(unused_crate_dependencies, clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::{NOPERM_PASSWORD, NOPERM_USER, PASSWORD, SIGNING_SECRET, USERNAME};
use serde_json::{Value, json};
use sqlx::query_scalar;
use uuid::Uuid;
use wicket_module::Profile;
use wicket_server::{capabilities, schema_bindings};

fn profiles() -> [Profile; 2] {
    [
        Profile::plain_shop().unwrap(),
        Profile::regulated_device().unwrap(),
    ]
}

fn mint_body(record_id: Uuid) -> serde_json::Value {
    json!({
        "meaning": "Approved",
        "record": {
            "table": "sm.instance",
            "id": record_id.to_string(),
            "version": 1
        },
        "identification": {
            "code": USERNAME,
            "secret": SIGNING_SECRET
        }
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn esign_http_under_both_profiles() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        let (st, ch) = w.post("/api/v1/esign/challenges", json!({})).await;
        assert_eq!(st, StatusCode::OK, "challenge {ch}");
        let comps = ch["components_required"].as_array().expect("components");
        assert!(
            comps.iter().any(|c| c.as_str() == Some("code"))
                && comps.iter().any(|c| c.as_str() == Some("secret")),
            "v1 two components, got {ch}"
        );
        assert_eq!(ch["credential_kind"], "signing_password");
        let challenge_back: wicket_esign::Challenge =
            serde_json::from_value(ch.clone()).expect("challenge crate type");
        assert_eq!(
            serde_json::to_value(&challenge_back).expect("challenge json"),
            ch,
            "esignChallenge HTTP is the crate Challenge wire"
        );

        let rec = Uuid::now_v7();
        let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
        assert_eq!(st, StatusCode::CREATED, "mint {minted}");
        let id = minted["signature"]["id"].as_str().expect("id");
        assert_eq!(minted["signature"]["meaning"], "Approved");
        assert_eq!(minted["signature"]["printed_name"], "M. Reyes");
        let minted_back: wicket_esign::Manifestation =
            serde_json::from_value(minted.clone()).expect("mint crate type");
        assert_eq!(
            serde_json::to_value(&minted_back).expect("mint json"),
            minted,
            "esignMint HTTP is the crate Manifestation wire"
        );

        let (st, got) = w.get(&format!("/api/v1/esign/signatures/{id}")).await;
        assert_eq!(st, StatusCode::OK, "get {got}");
        assert_eq!(got, minted, "getEsignSignature HTTP matches mint wire");

        let (st, bundle) = w
            .get(&format!("/api/v1/esign/signatures/{id}/bundle"))
            .await;
        assert_eq!(st, StatusCode::OK, "bundle {bundle}");
        assert_eq!(bundle["manifestation"]["signature"]["id"], id);
        assert!(
            bundle["record_content_hash"].is_array(),
            "bundle record_content_hash is [u8; 32] on the wire, got {bundle}"
        );
        let bundle_back: wicket_esign::ArchivalBundle =
            serde_json::from_value(bundle.clone()).expect("bundle crate type");
        assert_eq!(
            serde_json::to_value(&bundle_back).expect("bundle json"),
            bundle,
            "getEsignBundle HTTP is the crate ArchivalBundle wire"
        );

        let key = Uuid::now_v7().to_string();
        let rec2 = Uuid::now_v7();
        let (st, a) = w
            .post_key("/api/v1/esign/signatures", mint_body(rec2), &key, vec![])
            .await;
        assert_eq!(st, StatusCode::CREATED, "idempotent mint {a}");
        let (st, b) = w
            .post_key("/api/v1/esign/signatures", mint_body(rec2), &key, vec![])
            .await;
        assert_eq!(st, StatusCode::CREATED, "idempotent replay {b}");
        assert_eq!(a["signature"]["id"], b["signature"]["id"]);

        let (st, login_as_signing) = w
            .post(
                "/api/v1/esign/signatures",
                json!({
                    "meaning": "Approved",
                    "record": { "table": "sm.instance", "id": Uuid::now_v7().to_string(), "version": 1 },
                    "identification": { "code": USERNAME, "secret": PASSWORD }
                }),
            )
            .await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "login secret {login_as_signing}"
        );
        assert_eq!(
            login_as_signing["error"]["code"], "VALIDATION",
            "{login_as_signing}"
        );

        w.login_as(NOPERM_USER, NOPERM_PASSWORD).await;
        let (st, no_cred) = w
            .post(
                "/api/v1/esign/signatures",
                json!({
                    "meaning": "Approved",
                    "record": { "table": "sm.instance", "id": Uuid::now_v7().to_string(), "version": 1 },
                    "identification": { "code": NOPERM_USER, "secret": "not-a-signing-secret" }
                }),
            )
            .await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "no signing cred {no_cred}");
        assert_eq!(no_cred["error"]["code"], "SIGNATURE_REQUIRED", "{no_cred}");
        assert_eq!(
            no_cred["error"]["field"], "identification.secret",
            "{no_cred}"
        );
        w.login_as(USERNAME, PASSWORD).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn dummy_token_error_code_under_both_profiles() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let w = common::boot(profile.clone()).await;
        match profile.id {
            wicket_module::ProfileId::RegulatedDevice => {
                let cal = w.calibration_doc.as_ref().expect("calibration spawned");
                let (st, _, body) = w
                    .call(
                        "POST",
                        &format!("/api/v1/calibration/certificates/{cal}/approve"),
                        Some(vec![
                            ("if-match", "\"1\"".into()),
                            ("x-wicket-signature", Uuid::now_v7().to_string()),
                        ]),
                        Some(json!({})),
                    )
                    .await;
                assert_eq!(st, StatusCode::FORBIDDEN, "dummy {body}");
                assert_eq!(
                    body["error"]["code"], "SIGNATURE_REQUIRED",
                    "D-2b-5 Invalid while esign is bound, got {body}"
                );
                assert_ne!(
                    body["error"]["code"], "SIGNATURE_NO_PROVIDER",
                    "409 is NoProvider only"
                );
            }
            wicket_module::ProfileId::PlainShop => {
                let edges = &w.get("/api/v1/iq/manifest").await.1["signature_edges"];
                let required = edges
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e.get("Required").is_some() || e.get("required").is_some())
                    .count();
                assert_eq!(required, 0, "plain-shop has no Required edge {edges}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn regulated_release_refused_without_signature_succeeds_with_two_component_signature() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let cal = w
        .calibration_doc
        .clone()
        .expect("calibration.certificate spawned at boot");
    let rec = Uuid::parse_str(&cal).expect("uuid");
    let (st, refused) = w
        .post_if_match(
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            json!({}),
            1,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::UNAUTHORIZED,
        "refused without signature {refused}"
    );
    assert_eq!(refused["error"]["code"], "SIGNATURE_REQUIRED", "{refused}");

    let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");

    let (st, _, approved) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "signed approve {approved}");
    assert_eq!(
        approved,
        json!({"id": cal, "status": "approved"}),
        "approveCalibration wire must stay the frozen blob {approved}"
    );

    let w_plain = common::boot(Profile::plain_shop().unwrap()).await;
    assert!(
        w_plain.calibration_doc.is_none(),
        "plain-shop enables no regulated module"
    );
}

async fn quarantined_stock(w: &common::World) -> (String, String, String, i64) {
    let hex = Uuid::now_v7().simple().to_string().to_uppercase();
    let tag = &hex[..8];
    let item = {
        let (st, v) = w
            .post(
                "/api/v1/items",
                json!({
                    "number": format!("RM-SIG-{tag}"),
                    "revision": "A",
                    "description": "signature-edge bar",
                    "kind": "buy",
                    "stock_uom": 1,
                    "stock_scale": 0,
                    "residual_tolerance": "0",
                    "cost_method": "FIFO",
                }),
            )
            .await;
        assert_eq!(st, StatusCode::CREATED, "item {v}");
        v["id"].as_str().expect("item id").to_string()
    };
    let qloc = {
        let (st, v) = w
            .post(
                "/api/v1/locations",
                json!({"code": format!("WH-Q-{tag}"), "name": "Quarantine", "kind": "warehouse"}),
            )
            .await;
        assert_eq!(st, StatusCode::CREATED, "qloc {v}");
        v["id"].as_str().expect("qloc").to_string()
    };
    let aloc = {
        let (st, v) = w
            .post(
                "/api/v1/locations",
                json!({"code": format!("WH-A-{tag}"), "name": "Available", "kind": "warehouse"}),
            )
            .await;
        assert_eq!(st, StatusCode::CREATED, "aloc {v}");
        v["id"].as_str().expect("aloc").to_string()
    };
    let lot = {
        let (st, v) = w
            .post(
                "/api/v1/lots",
                json!({"item_id": item, "identifier": format!("LOT-SIG-{tag}")}),
            )
            .await;
        assert_eq!(st, StatusCode::CREATED, "lot {v}");
        v["id"].as_str().expect("lot").to_string()
    };
    let (st, rec) = w
        .post(
            "/api/v1/inventory/receipts",
            json!({
                "item_id": item,
                "lot_id": lot,
                "location_id": qloc,
                "quantity": {"amount": "100", "unit": 1, "dimension": "Count"},
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "receipt {rec}");
    let version = w.get(&format!("/api/v1/lots/{lot}")).await.1["version"]
        .as_i64()
        .unwrap_or(1);
    (lot, qloc, aloc, version)
}

fn mint_lot_body(record_id: &str, version: i64) -> serde_json::Value {
    json!({
        "meaning": "Lot released",
        "record": {
            "table": "sm.instance",
            "id": record_id,
            "version": version
        },
        "identification": {
            "code": USERNAME,
            "secret": SIGNING_SECRET
        }
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn lot_release_without_signature_is_403_regulated() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let (lot, qloc, aloc, ver) = quarantined_stock(&w).await;
    let (st, body) = w
        .post_if_match(
            "/api/v1/inventory/releases",
            json!({
                "lot_id": lot,
                "from_location_id": qloc,
                "to_location_id": aloc,
                "entered": {"amount": "100", "unit": 1, "dimension": "Count"},
            }),
            ver,
        )
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "unsigned release {body}");
    assert_eq!(body["error"]["code"], "SIGNATURE_REQUIRED", "{body}");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("missing token"),
        "D-2b-5 missing token, got {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn lot_release_consumes_minted_signature_regulated() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let (lot, qloc, aloc, ver) = quarantined_stock(&w).await;
    let (st, minted) = w
        .post("/api/v1/esign/signatures", mint_lot_body(&lot, ver))
        .await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");

    let (st, _, released) = w
        .call(
            "POST",
            "/api/v1/inventory/releases",
            Some(vec![
                ("if-match", format!("\"{ver}\"")),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({
                "lot_id": lot,
                "from_location_id": qloc,
                "to_location_id": aloc,
                "entered": {"amount": "100", "unit": 1, "dimension": "Count"},
            })),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "signed release {released}");
    let after = w.get(&format!("/api/v1/lots/{lot}")).await.1;
    assert_eq!(after["status"], "available", "{after}");

    let (st, _, replay) = w
        .call(
            "POST",
            "/api/v1/inventory/releases",
            Some(vec![
                ("if-match", format!("\"{ver}\"")),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({
                "lot_id": lot,
                "from_location_id": qloc,
                "to_location_id": aloc,
                "entered": {"amount": "100", "unit": 1, "dimension": "Count"},
            })),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "replay {replay}");
    assert_eq!(replay["error"]["code"], "CONFLICT", "{replay}");
}

#[tokio::test(flavor = "multi_thread")]
async fn lot_release_plain_shop_needs_no_signature() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::plain_shop().unwrap()).await;
    let (lot, qloc, aloc, ver) = quarantined_stock(&w).await;
    let (st, released) = w
        .post_if_match(
            "/api/v1/inventory/releases",
            json!({
                "lot_id": lot,
                "from_location_id": qloc,
                "to_location_id": aloc,
                "entered": {"amount": "100", "unit": 1, "dimension": "Count"},
            }),
            ver,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "plain-shop release {released}");
    let after = w.get(&format!("/api/v1/lots/{lot}")).await.1;
    assert_eq!(after["status"], "available", "{after}");
}

#[tokio::test(flavor = "multi_thread")]
async fn lot_status_required_edge_consumes_signature() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let (lot, _qloc, _aloc, ver) = quarantined_stock(&w).await;
    let (st, minted) = w
        .post("/api/v1/esign/signatures", mint_lot_body(&lot, ver))
        .await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");

    let (st, _, body) = w
        .call(
            "POST",
            &format!("/api/v1/lots/{lot}/status"),
            Some(vec![
                ("if-match", format!("\"{ver}\"")),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({"status": "available", "reason": "signed release"})),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "signed status {body}");
    assert_eq!(body["status"], "available", "{body}");

    let (st, _, replay) = w
        .call(
            "POST",
            &format!("/api/v1/lots/{lot}/status"),
            Some(vec![
                ("if-match", format!("\"{ver}\"")),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({"status": "available", "reason": "replay"})),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "replay {replay}");
    assert_eq!(replay["error"]["code"], "CONFLICT", "{replay}");
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_signature_header_reports_missing_token() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let cal = w.calibration_doc.as_ref().expect("calibration spawned");
    let (st, body) = w
        .post_if_match(
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            json!({}),
            1,
        )
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "missing header {body}");
    assert_eq!(body["error"]["code"], "SIGNATURE_REQUIRED", "{body}");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("missing token"),
        "wire message must be Invalid(missing token), got {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn calibration_replay_of_consumed_signature_is_409() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let cal = w
        .calibration_doc
        .clone()
        .expect("calibration.certificate spawned at boot");
    let rec = Uuid::parse_str(&cal).expect("uuid");
    let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");

    let (st, _, approved) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "signed approve {approved}");

    let (st, _, replay) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "replay {replay}");
    assert_eq!(replay["error"]["code"], "CONFLICT", "{replay}");
    assert_ne!(replay["error"]["code"], "INTERNAL", "{replay}");
}

#[tokio::test(flavor = "multi_thread")]
async fn consumed_signature_replay_is_409_via_published_seam() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let cal = w
        .calibration_doc
        .clone()
        .expect("calibration.certificate spawned at boot");
    let rec = Uuid::parse_str(&cal).expect("uuid");
    let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");

    let (st, _, approved) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "signed approve {approved}");

    let (st, _, replay) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "replay {replay}");
    assert_eq!(replay["error"]["code"], "CONFLICT", "{replay}");
    assert_ne!(replay["error"]["code"], "INTERNAL", "{replay}");
}

#[tokio::test(flavor = "multi_thread")]
async fn server_manifest_matches_crate_wire_after_version_bump() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;
    let cal = w
        .calibration_doc
        .clone()
        .expect("calibration.certificate spawned at boot");
    let rec = Uuid::parse_str(&cal).expect("uuid");
    let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
    assert_eq!(st, StatusCode::CREATED, "mint {minted}");
    let sig_id = minted["signature"]["id"].as_str().expect("id");
    assert_eq!(minted["signature"]["superseded"], false, "{minted}");
    assert!(
        minted["signature"]["superseded_by_version"].is_null(),
        "{minted}"
    );

    let (st, _, approved) = w
        .call(
            "POST",
            &format!("/api/v1/calibration/certificates/{cal}/approve"),
            Some(vec![
                ("if-match", "\"1\"".into()),
                ("x-wicket-signature", sig_id.to_string()),
            ]),
            Some(json!({})),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "bump via approve {approved}");

    let (st, got) = w.get(&format!("/api/v1/esign/signatures/{sig_id}")).await;
    assert_eq!(st, StatusCode::OK, "get {got}");
    assert_eq!(got["signature"]["superseded"], true, "{got}");
    let live = got["signature"]["superseded_by_version"]
        .as_i64()
        .expect("superseded_by_version");
    let record_version = got["signature"]["record"]["version"]
        .as_i64()
        .expect("record.version");
    assert!(
        live > record_version,
        "live {live} must exceed record {record_version}: {got}"
    );

    let sid = wicket_core::SignatureId::from_uuid(Uuid::parse_str(sig_id).expect("uuid"));
    let crate_wire = wicket_esign::manifestation(&wicket_db::ReadPool::new(w.pool.clone()), sid)
        .await
        .expect("crate manifestation");
    let crate_json = serde_json::to_value(&crate_wire).expect("crate json");
    assert_eq!(got, crate_json, "HTTP GET is the crate D-2b-2 wire");
}

#[tokio::test(flavor = "multi_thread")]
async fn http_mint_snapshot_matches_target_required_edge() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::regulated_device().unwrap()).await;

    let (st, doc) = w
        .post(
            "/api/v1/documents",
            json!({
                "kind": "SOP",
                "title": "snapshot target",
                "retention_class": "quality",
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "create document {doc}");
    let doc_id = doc["id"].as_str().expect("id");
    let doc_ver = doc["version"].as_i64().unwrap_or(1);
    let (st, minted_doc) = w
        .post(
            "/api/v1/esign/signatures",
            json!({
                "meaning": "Approved",
                "record": {
                    "table": "sm.instance",
                    "id": doc_id,
                    "version": doc_ver
                },
                "identification": { "code": USERNAME, "secret": SIGNING_SECRET }
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "document mint {minted_doc}");
    let doc_sig = minted_doc["signature"]["id"].as_str().expect("id");
    let doc_snap: Vec<String> =
        query_scalar("SELECT permission_snapshot FROM esign.signature WHERE signature_id = $1")
            .bind(Uuid::parse_str(doc_sig).expect("uuid"))
            .fetch_one(&w.pool)
            .await
            .expect("document snapshot");
    assert!(
        doc_snap.iter().any(|k| k == "documents.approve"),
        "document mint snapshots documents.approve, got {doc_snap:?}"
    );

    let cal = w
        .calibration_doc
        .clone()
        .expect("calibration.certificate spawned at boot");
    let (st, minted_cal) = w
        .post(
            "/api/v1/esign/signatures",
            mint_body(Uuid::parse_str(&cal).expect("uuid")),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "calibration mint {minted_cal}");
    let cal_sig = minted_cal["signature"]["id"].as_str().expect("id");
    let cal_snap: Vec<String> =
        query_scalar("SELECT permission_snapshot FROM esign.signature WHERE signature_id = $1")
            .bind(Uuid::parse_str(cal_sig).expect("uuid"))
            .fetch_one(&w.pool)
            .await
            .expect("calibration snapshot");
    assert!(
        cal_snap.iter().any(|k| k == "calibration.approve"),
        "calibration mint snapshots calibration.approve, got {cal_snap:?}"
    );
}

#[test]
fn esign_mint_remembers_in_the_mint_transaction() {
    let src = include_str!("../src/handlers/mod.rs");
    let start = src
        .find("async fn esign_mint_inner")
        .expect("esign_mint_inner");
    let rest = &src[start..];
    let end = rest
        .find("\n/// GET /api/v1/esign/signatures/{id}")
        .unwrap_or(rest.len());
    let body = &rest[..end];
    let begins = body.matches("Tx::begin(").count();
    assert_eq!(begins, 1, "mint + remember share one Tx, found {begins}");
    let mint = body.find("wicket_esign::mint").expect("mint call");
    let after_mint = &body[mint..];
    assert!(
        !after_mint.contains("Tx::begin("),
        "no follow-up Tx after mint"
    );
    let remember = after_mint.find("idempotency::remember").expect("remember");
    let commit = after_mint.find("tx.commit()").expect("commit after mint");
    assert!(
        remember < commit,
        "remember must run before commit in the mint Tx"
    );
}

#[test]
fn server_src_does_not_select_esign_schema() {
    let handlers = include_str!("../src/handlers/mod.rs");
    assert!(
        !handlers.contains("include_str!(\"esign_"),
        "sql includes must be deleted"
    );
    assert!(
        !handlers.contains("FROM esign."),
        "no FROM esign.* in handlers"
    );
    assert!(handlers.contains("wicket_esign::signature_consumed_at"));
    assert!(handlers.contains("wicket_esign::manifestation_in_tx"));
    assert!(handlers.contains("wicket_esign::manifestation("));
}

#[test]
fn esign_schema_bindings_cover_the_five_operations() {
    let registered = schema_bindings();
    let want = [
        ("approveCalibration", false),
        ("esignChallenge", false),
        ("esignMint", true),
        ("getEsignSignature", false),
        ("getEsignBundle", false),
    ];
    for (id, has_request) in want {
        assert!(
            registered
                .iter()
                .any(|(got, req)| *got == id && *req == has_request),
            "{id} has_request={has_request} missing from {registered:?}"
        );
    }
}

fn served_op<'a>(doc: &'a Value, id: &str) -> &'a Value {
    let cap = capabilities()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("{id} missing from capability table"));
    let op = &doc["paths"][cap.path][cap.method.to_ascii_lowercase()];
    assert!(op.is_object(), "missing {id} in served document");
    assert_eq!(op["operationId"].as_str(), Some(id));
    op
}

#[tokio::test(flavor = "multi_thread")]
async fn served_openapi_types_esign_operations() {
    if common::skip_if_no_pg() {
        return;
    }
    let w = common::boot(Profile::plain_shop().unwrap()).await;
    let (st, doc) = w.get("/api/v1/openapi.json").await;
    assert_eq!(st, StatusCode::OK, "{doc}");

    let challenge = served_op(&doc, "esignChallenge");
    assert_eq!(
        challenge["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Challenge"
    );
    assert!(challenge.get("requestBody").is_none());
    assert!(
        challenge["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|p| p["name"] != "Idempotency-Key"),
        "esignChallenge is not on the idempotency list {challenge}"
    );

    let mint = served_op(&doc, "esignMint");
    assert_eq!(
        mint["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/EsignMintBody"
    );
    assert_eq!(
        mint["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Manifestation"
    );
    assert!(
        mint["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "Idempotency-Key"),
        "esignMint Idempotency-Key {mint}"
    );

    let get_sig = served_op(&doc, "getEsignSignature");
    assert_eq!(
        get_sig["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Manifestation"
    );
    assert!(
        get_sig["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "id" && p["in"] == "path"),
        "getEsignSignature path id {get_sig}"
    );

    let bundle = served_op(&doc, "getEsignBundle");
    assert_eq!(
        bundle["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ArchivalBundle"
    );
    assert!(
        bundle["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "id" && p["in"] == "path"),
        "getEsignBundle path id {bundle}"
    );

    let approve = served_op(&doc, "approveCalibration");
    assert_eq!(
        approve["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/CalibrationApprovedBody"
    );
    assert!(approve.get("requestBody").is_none());
    assert!(
        approve["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "id" && p["in"] == "path"),
        "approveCalibration path id {approve}"
    );
    assert!(
        approve["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "If-Match"),
        "approveCalibration If-Match {approve}"
    );
    assert!(
        approve["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["name"] == "Idempotency-Key"),
        "approveCalibration Idempotency-Key {approve}"
    );

    assert!(doc["components"]["schemas"]["Challenge"].is_object());
    assert!(doc["components"]["schemas"]["EsignMintBody"].is_object());
    assert!(doc["components"]["schemas"]["Manifestation"].is_object());
    assert!(doc["components"]["schemas"]["ArchivalBundle"].is_object());
    assert!(doc["components"]["schemas"]["CalibrationApprovedBody"].is_object());
    let bundle_hash =
        &doc["components"]["schemas"]["ArchivalBundle"]["properties"]["record_content_hash"];
    assert_eq!(
        bundle_hash["type"], "array",
        "bundle hash is [u8; 32] on the wire, not hex: {bundle_hash}"
    );
    assert_eq!(bundle_hash["minItems"], 32);
    assert_eq!(bundle_hash["maxItems"], 32);
    let snapshot = &doc["components"]["schemas"]["ArchivalBundle"]["properties"]["record_snapshot"];
    assert!(
        snapshot.is_object() && snapshot != &json!(true),
        "record_snapshot is any JSON, as an OpenAPI schema object: {snapshot}"
    );
    assert_eq!(
        doc["components"]["schemas"]["Manifestation"]["properties"]["signature"]["allOf"][0]["$ref"],
        "#/components/schemas/SignatureManifest"
    );
    assert_eq!(
        doc["components"]["schemas"]["SignatureManifest"]["properties"]["record_content_hash"]["type"],
        "string",
        "manifestation record_content_hash is hex, not the bundle's byte array"
    );
    assert_eq!(
        doc["components"]["schemas"]["CalibrationApprovedBody"]["properties"]["status"]["enum"],
        json!(["approved"])
    );
    assert!(doc["components"]["schemas"]["EsignRecordBody"].is_object());
    assert!(doc["components"]["schemas"]["EsignIdentBody"].is_object());
    assert!(doc["components"]["schemas"]["SealRef"].is_object());
    assert!(doc["components"]["schemas"]["AnchorRef"].is_object());
    assert!(doc["components"]["schemas"]["ManifestRecord"].is_object());
}

/// Canonicalize volatile ids, timestamps, hex digests, and byte arrays so two
/// boots of the same handlers compare equal. Set `WICKET_ESIGN_CAPTURE` to a
/// file path to write the bodies; unset, this test does nothing.
#[tokio::test(flavor = "multi_thread")]
async fn capture_esign_wire_when_requested() {
    let Ok(path) = std::env::var("WICKET_ESIGN_CAPTURE") else {
        return;
    };
    if common::skip_if_no_pg() {
        panic!("WICKET_ESIGN_CAPTURE set but postgres is unavailable");
    }

    let mut out = serde_json::Map::new();
    for profile in profiles() {
        let w = common::boot(profile.clone()).await;
        let mut slot = serde_json::Map::new();
        let (st, ch) = w.post("/api/v1/esign/challenges", json!({})).await;
        assert_eq!(st, StatusCode::OK, "challenge {ch}");
        slot.insert("esignChallenge".into(), canon_op(st, &ch));

        let rec = Uuid::now_v7();
        let (st, minted) = w.post("/api/v1/esign/signatures", mint_body(rec)).await;
        assert_eq!(st, StatusCode::CREATED, "mint {minted}");
        slot.insert("esignMint".into(), canon_op(st, &minted));
        let id = minted["signature"]["id"].as_str().expect("id").to_string();

        let (st, got) = w.get(&format!("/api/v1/esign/signatures/{id}")).await;
        assert_eq!(st, StatusCode::OK, "get {got}");
        slot.insert("getEsignSignature".into(), canon_op(st, &got));

        let (st, bundle) = w
            .get(&format!("/api/v1/esign/signatures/{id}/bundle"))
            .await;
        assert_eq!(st, StatusCode::OK, "bundle {bundle}");
        slot.insert("getEsignBundle".into(), canon_op(st, &bundle));

        if let Some(cal) = w.calibration_doc.clone() {
            let (st, minted_cal) = w
                .post(
                    "/api/v1/esign/signatures",
                    mint_body(Uuid::parse_str(&cal).expect("cal uuid")),
                )
                .await;
            assert_eq!(st, StatusCode::CREATED, "cal mint {minted_cal}");
            let sig_id = minted_cal["signature"]["id"]
                .as_str()
                .expect("sig")
                .to_string();
            let (st, _, approved) = w
                .call(
                    "POST",
                    &format!("/api/v1/calibration/certificates/{cal}/approve"),
                    Some(vec![
                        ("if-match", "\"1\"".into()),
                        ("x-wicket-signature", sig_id),
                    ]),
                    Some(json!({})),
                )
                .await;
            assert_eq!(st, StatusCode::OK, "approve {approved}");
            slot.insert("approveCalibration".into(), canon_op(st, &approved));
        }

        let (st, doc) = w.get("/api/v1/openapi.json").await;
        assert_eq!(st, StatusCode::OK, "openapi");
        let mut schemas = serde_json::Map::new();
        for id in [
            "approveCalibration",
            "esignChallenge",
            "esignMint",
            "getEsignSignature",
            "getEsignBundle",
        ] {
            let op = capture_served_op(&doc, id);
            schemas.insert(
                id.into(),
                json!({
                    "requestBody": op.get("requestBody").cloned().unwrap_or(Value::Null),
                    "response200": op["responses"]["200"]["content"].clone(),
                    "parameters": op.get("parameters").cloned().unwrap_or(Value::Null),
                }),
            );
        }
        let openapi_path = format!("{path}.{}.openapi.json", profile_key(&profile));
        std::fs::write(
            &openapi_path,
            serde_json::to_string_pretty(&schemas).expect("openapi json"),
        )
        .expect("write openapi slice");

        out.insert(profile_key(&profile).into(), Value::Object(slot));
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&out).expect("wire json"),
    )
    .expect("write wire");
}

fn capture_served_op<'a>(doc: &'a Value, id: &str) -> &'a Value {
    let cap = wicket_server::capabilities()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("{id} missing from capability table"));
    let op = &doc["paths"][cap.path][cap.method.to_ascii_lowercase()];
    assert!(op.is_object(), "missing {id} in served document");
    op
}

fn profile_key(profile: &Profile) -> &'static str {
    match profile.id {
        wicket_module::ProfileId::PlainShop => "plain-shop",
        wicket_module::ProfileId::RegulatedDevice => "regulated-device",
    }
}

fn canon_op(status: StatusCode, body: &Value) -> Value {
    json!({"status": status.as_u16(), "body": canon(body)})
}

fn canon(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(canon_str(s)),
        Value::Array(items) => {
            if !items.is_empty() && items.iter().all(Value::is_number) {
                json!({"$bytes": items.len()})
            } else {
                Value::Array(items.iter().map(canon).collect())
            }
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                // Postgres transaction ids change every mint. The field stays.
                if k == "xid" {
                    out.insert(k.clone(), Value::String("XID".into()));
                } else {
                    out.insert(k.clone(), canon(val));
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn canon_str(s: &str) -> String {
    if is_uuid(s) {
        return "UUID".into();
    }
    if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return "HEX".into();
    }
    if is_rfc3339(s) {
        return "TS".into();
    }
    s.to_string()
}

fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (i, c) in b.iter().enumerate() {
        let dash = matches!(i, 8 | 13 | 18 | 23);
        if dash && *c != b'-' {
            return false;
        }
        if !dash && !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_rfc3339(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || b.len() > 40 || !b[0].is_ascii_digit() || !s.contains('T') {
        return false;
    }
    b.iter()
        .all(|c| c.is_ascii_digit() || matches!(*c, b'-' | b':' | b'T' | b'Z' | b'+' | b'.'))
}
