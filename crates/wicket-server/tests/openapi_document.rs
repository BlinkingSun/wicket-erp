//! Served `GET /api/v1/openapi.json` equals the committed profile fixtures.
//!
//! Compare parsed JSON values, not bytes. The HTTP body is compact and the
//! fixture is pretty. Regeneration is `just openapi-document`.

#![allow(unused_crate_dependencies, clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use serde_json::Value;
use wicket_module::{Profile, ProfileId};

fn profiles() -> [Profile; 2] {
    [
        Profile::plain_shop().unwrap(),
        Profile::regulated_device().unwrap(),
    ]
}

fn fixture(id: ProfileId) -> Value {
    let raw = match id {
        ProfileId::PlainShop => include_str!("fixtures/openapi-document.json"),
        ProfileId::RegulatedDevice => include_str!("fixtures/openapi-document-regulated.json"),
    };
    serde_json::from_str(raw).expect("committed openapi fixture")
}

#[tokio::test(flavor = "multi_thread")]
async fn served_openapi_document_matches_committed_fixture() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let label = profile.id.as_str();
        let expected = fixture(profile.id);
        let w = common::boot(profile).await;
        let (status, doc) = w.get("/api/v1/openapi.json").await;
        assert_eq!(status, StatusCode::OK, "{label} openapi status");
        assert_eq!(
            doc, expected,
            "{label} served document differs from fixture"
        );
    }
}
