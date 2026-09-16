//! Mount wave 1: identity reads and by-number resolution over HTTP.

#![allow(
    unused_crate_dependencies,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_arguments
)]

mod common;

use axum::http::StatusCode;
use common::World;
use serde_json::{Value, json};
use wicket_module::Profile;

fn profiles() -> [Profile; 2] {
    [
        Profile::plain_shop().unwrap(),
        Profile::regulated_device().unwrap(),
    ]
}

async fn create_item(w: &World, number: &str) -> String {
    let (st, v) = w
        .post(
            "/api/v1/items",
            json!({
                "number": number,
                "revision": "A",
                "description": number,
                "kind": "buy",
                "stock_uom": 1,
                "stock_scale": 0,
                "residual_tolerance": "0",
                "cost_method": "FIFO",
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "create {number} {v}");
    v["id"].as_str().unwrap().to_string()
}

async fn release_wo_number(w: &World, item: &str) -> (String, String) {
    let (st, wo) = w
        .post(
            "/api/v1/work-orders",
            json!({
                "item_id": item,
                "revision": "A",
                "quantity": common::qty("1", 1, "Count"),
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "wo {wo}");
    let wo_id = wo["id"].as_str().unwrap().to_string();
    let ver = wo["version"].as_i64().unwrap_or(1);
    let (st, wo) = w
        .post_if_match(
            &format!("/api/v1/work-orders/{wo_id}/release"),
            json!({}),
            ver,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "wo release {wo}");
    let number = wo["number"]
        .as_str()
        .expect("released work order has a number")
        .to_string();
    (wo_id, number)
}

fn assert_list_envelope(body: &Value, label: &str) {
    assert!(body["data"].is_array(), "{label} data {body}");
    assert!(body["has_more"].is_boolean(), "{label} has_more {body}");
    if body["has_more"].as_bool().unwrap() {
        assert!(
            body["next_cursor"].is_string(),
            "{label} next_cursor {body}"
        );
    } else {
        assert!(
            body["next_cursor"].is_null(),
            "{label} next_cursor null {body}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn list_principals_happy_and_invalid_limit() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        let (st, body) = w.get("/api/v1/identity/principals?limit=1").await;
        assert_eq!(st, StatusCode::OK, "listPrincipals {body}");
        assert_list_envelope(&body, "listPrincipals");
        assert!(!body["data"].as_array().unwrap().is_empty(), "{body}");
        let (st, body) = w.get("/api/v1/identity/principals?limit=201").await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "invalid limit {body}");
        assert_eq!(body["error"]["code"], "VALIDATION", "{body}");
        assert_eq!(body["error"]["field"], "limit", "{body}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn get_principal_by_username_happy() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        let (st, body) = w
            .get("/api/v1/identity/principals/by-username?username=mreyes")
            .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["username"], "mreyes", "{body}");
        let (st, body) = w
            .get("/api/v1/identity/principals/by-username?username=no-such-user")
            .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"]["code"], "NOT_FOUND", "{body}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn list_roles_happy_and_invalid_limit() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        let (st, body) = w.get("/api/v1/identity/roles").await;
        assert_eq!(st, StatusCode::OK, "listRoles {body}");
        assert_list_envelope(&body, "listRoles");
        assert!(!body["data"].as_array().unwrap().is_empty(), "{body}");
        let (st, body) = w.get("/api/v1/identity/roles?limit=0").await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "invalid limit {body}");
        assert_eq!(body["error"]["code"], "VALIDATION", "{body}");
        assert_eq!(body["error"]["field"], "limit", "{body}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn get_role_by_name_happy() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        let (st, body) = w
            .get("/api/v1/identity/roles/by-name?name=slice-operator")
            .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["name"], "slice-operator", "{body}");
        assert!(
            body["permissions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p == "items.view")
        );
        let (st, body) = w
            .get("/api/v1/identity/roles/by-name?name=no-such-role")
            .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"]["code"], "NOT_FOUND", "{body}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn list_roles_for_principal_happy() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let mut w = common::boot(profile).await;
        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        let (st, me) = w.get("/api/v1/identity/me").await;
        assert_eq!(st, StatusCode::OK, "{me}");
        let id = me["id"].as_str().unwrap();
        let (st, body) = w
            .get(&format!("/api/v1/identity/principals/{id}/roles"))
            .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_list_envelope(&body, "listRolesForPrincipal");
        assert!(
            body["data"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["name"] == "slice-identity-admin"),
            "{body}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn resolve_item_by_number_happy_and_unknown_is_404() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let w = common::boot(profile).await;
        let id = create_item(&w, "MDS-450-SCAN").await;
        let (st, body) = w.get("/api/v1/items/resolve?number=MDS-450-SCAN").await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["id"], id, "{body}");
        let (st, body) = w.get("/api/v1/items/resolve?number=MDS-NO-SUCH").await;
        assert_eq!(st, StatusCode::NOT_FOUND, "unknown number {body}");
        assert_eq!(body["error"]["code"], "NOT_FOUND", "{body}");
        assert_ne!(st, StatusCode::INTERNAL_SERVER_ERROR);
        let (st, body) = w.get("/api/v1/items/resolve?number=mds-450-scan").await;
        assert_eq!(
            st,
            StatusCode::NOT_FOUND,
            "case-sensitive miss must be 404 {body}"
        );
        assert_eq!(body["error"]["code"], "NOT_FOUND", "{body}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn resolve_work_order_by_number_happy_and_unknown_is_404() {
    if common::skip_if_no_pg() {
        return;
    }
    for profile in profiles() {
        let w = common::boot(profile).await;
        let item = create_item(&w, "WO-RES-ITEM").await;
        let (wo_id, number) = release_wo_number(&w, &item).await;
        let (st, body) = w
            .get(&format!("/api/v1/work-orders/resolve?number={number}"))
            .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["id"], wo_id, "{body}");
        let (st, body) = w.get("/api/v1/work-orders/resolve?number=WO-NO-SUCH").await;
        assert_eq!(st, StatusCode::NOT_FOUND, "unknown number {body}");
        assert_eq!(body["error"]["code"], "NOT_FOUND", "{body}");
        assert_ne!(st, StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn seven_operations_are_closed_world_on_permission() {
    if common::skip_if_no_pg() {
        return;
    }
    const IDENTITY: &[&str] = &[
        "/api/v1/identity/principals",
        "/api/v1/identity/principals/by-username?username=mreyes",
        "/api/v1/identity/roles",
        "/api/v1/identity/roles/by-name?name=slice-operator",
        "/api/v1/identity/principals/00000000-0000-0000-0000-000000000001/roles",
    ];
    const RESOLVE: &[&str] = &[
        "/api/v1/items/resolve?number=MDS-NO-SUCH",
        "/api/v1/work-orders/resolve?number=WO-NO-SUCH",
    ];
    for profile in profiles() {
        let mut w = common::boot(profile).await;

        w.login_as(common::NOPERM_USER, common::NOPERM_PASSWORD)
            .await;
        for uri in IDENTITY.iter().chain(RESOLVE) {
            let (st, body) = w.get(uri).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "noperm {uri} {body}");
            assert_eq!(body["error"]["code"], "FORBIDDEN", "{uri} {body}");
        }

        w.login_as(common::USERNAME, common::PASSWORD).await;
        for uri in IDENTITY {
            let (st, body) = w.get(uri).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "operator {uri} {body}");
            assert_eq!(body["error"]["code"], "FORBIDDEN", "{uri} {body}");
        }
        for uri in RESOLVE {
            let (st, body) = w.get(uri).await;
            assert_eq!(
                st,
                StatusCode::NOT_FOUND,
                "operator may call resolver {uri} {body}"
            );
            assert_eq!(body["error"]["code"], "NOT_FOUND", "{uri} {body}");
        }

        w.login_as(common::ADMIN_USER, common::ADMIN_PASSWORD).await;
        for uri in IDENTITY {
            let (st, body) = w.get(uri).await;
            assert!(
                st == StatusCode::OK || st == StatusCode::NOT_FOUND,
                "admin identity {uri} {st} {body}"
            );
        }
        for uri in RESOLVE {
            let (st, body) = w.get(uri).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "admin {uri} {body}");
            assert_eq!(body["error"]["code"], "FORBIDDEN", "{uri} {body}");
        }
    }
}
