//! Admin enumeration reads (W3a-bis).

#![allow(clippy::unwrap_used, clippy::expect_used, unused_crate_dependencies)]

mod common;

use serde_json::Value;
use wicket_db::Tx;
use wicket_identity::rbac::assign_role;
use wicket_identity::{
    Error, PrincipalKind, create_principal, list_principals, list_roles, list_roles_for_principal,
    load_bundles, load_principal_by_username, load_role_by_name, seed_bundles,
};
use wicket_test::db_case;

use common::{migrate_identity, system_ctx, write_pool};

fn assert_no_credential_fields(value: &Value) {
    let json = value.to_string().to_lowercase();
    assert!(
        !json.contains("hash"),
        "response must not include credential hash"
    );
    assert!(
        !json.contains("argon2"),
        "response must not leak PHC material"
    );
}

#[tokio::test]
async fn list_principals_pagination_and_invalid_limit() {
    let db = db_case!("id_list_principals");
    migrate_identity(&db).await;
    let write = write_pool(&db).await;
    let mut tx = Tx::begin(&write, &system_ctx("identity.create"))
        .await
        .expect("begin");
    create_principal(&mut tx, PrincipalKind::User, "page-a", "A")
        .await
        .expect("a");
    create_principal(&mut tx, PrincipalKind::User, "page-b", "B")
        .await
        .expect("b");
    tx.commit().await.expect("commit");

    let mut tx = Tx::begin(&write, &system_ctx("identity.read"))
        .await
        .expect("read");
    let err = list_principals(&mut tx, Some(201), None, None, None)
        .await
        .expect_err("limit 201");
    assert!(matches!(err, Error::InvalidLimit));

    let page1 = list_principals(&mut tx, Some(1), None, None, None)
        .await
        .expect("page1");
    assert_eq!(page1.data.len(), 1);
    assert!(page1.has_more);
    let cursor = page1.next_cursor.expect("cursor");
    let page2 = list_principals(&mut tx, Some(1), Some(&cursor), None, None)
        .await
        .expect("page2");
    assert_eq!(page2.data.len(), 1);
    assert_ne!(page1.data[0].id, page2.data[0].id);

    for p in &page1.data {
        let v = serde_json::to_value(p).expect("json");
        assert_no_credential_fields(&v);
    }
    tx.commit().await.expect("commit read");
    db.finish().await.expect("finish");
}

#[tokio::test]
async fn load_principal_by_username_case_insensitive() {
    let db = db_case!("id_load_username");
    migrate_identity(&db).await;
    let write = write_pool(&db).await;
    let mut tx = Tx::begin(&write, &system_ctx("identity.create"))
        .await
        .expect("begin");
    create_principal(&mut tx, PrincipalKind::User, "MixedCase", "Mixed")
        .await
        .expect("create");
    tx.commit().await.expect("commit");

    let mut tx = Tx::begin(&write, &system_ctx("identity.read"))
        .await
        .expect("read");
    let p = load_principal_by_username(&mut tx, "mixedcase")
        .await
        .expect("lookup");
    assert_eq!(p.username, "MixedCase");
    let v = serde_json::to_value(&p).expect("json");
    assert_no_credential_fields(&v);
    tx.commit().await.expect("commit read");
    db.finish().await.expect("finish");
}

#[tokio::test]
async fn list_roles_pagination_and_permissions_populated() {
    let db = db_case!("id_list_roles");
    migrate_identity(&db).await;
    let write = write_pool(&db).await;
    let bundles = load_bundles(include_str!("../fixtures/roles.toml")).expect("toml");
    let mut tx = Tx::begin(&write, &system_ctx("identity.rbac"))
        .await
        .expect("begin");
    seed_bundles(&mut tx, &bundles).await.expect("seed");
    tx.commit().await.expect("commit");

    let mut tx = Tx::begin(&write, &system_ctx("identity.read"))
        .await
        .expect("read");
    let err = list_roles(&mut tx, Some(0), None)
        .await
        .expect_err("limit 0");
    assert!(matches!(err, Error::InvalidLimit));

    let page = list_roles(&mut tx, None, None).await.expect("all");
    assert!(page.data.len() >= 2);
    for role in &page.data {
        assert!(
            !role.permissions.is_empty(),
            "role {} must include permissions",
            role.name
        );
    }
    let admin = page
        .data
        .iter()
        .find(|r| r.name == "admin")
        .expect("admin role");
    assert!(
        admin
            .permissions
            .contains(&"calibration.approve".to_string())
    );
    tx.commit().await.expect("commit read");
    db.finish().await.expect("finish");
}

#[tokio::test]
async fn load_role_by_name_permissions_populated() {
    let db = db_case!("id_load_role");
    migrate_identity(&db).await;
    let write = write_pool(&db).await;
    let bundles = load_bundles(include_str!("../fixtures/roles.toml")).expect("toml");
    let mut tx = Tx::begin(&write, &system_ctx("identity.rbac"))
        .await
        .expect("begin");
    seed_bundles(&mut tx, &bundles).await.expect("seed");
    tx.commit().await.expect("commit");

    let mut tx = Tx::begin(&write, &system_ctx("identity.read"))
        .await
        .expect("read");
    let operator = load_role_by_name(&mut tx, "operator")
        .await
        .expect("operator");
    assert!(
        operator
            .permissions
            .contains(&"calibration.view".to_string())
    );
    assert!(
        operator
            .permissions
            .contains(&"identity.session".to_string())
    );
    tx.commit().await.expect("commit read");
    db.finish().await.expect("finish");
}

#[tokio::test]
async fn list_roles_for_principal_permissions_and_zero_roles() {
    let db = db_case!("id_roles_for_principal");
    migrate_identity(&db).await;
    let write = write_pool(&db).await;
    let bundles = load_bundles(include_str!("../fixtures/roles.toml")).expect("toml");
    let mut tx = Tx::begin(&write, &system_ctx("identity.rbac"))
        .await
        .expect("begin");
    let lonely = create_principal(&mut tx, PrincipalKind::User, "noroles", "No Roles")
        .await
        .expect("lonely");
    let with_role = create_principal(&mut tx, PrincipalKind::User, "hasrole", "Has Role")
        .await
        .expect("hasrole");
    let ids = seed_bundles(&mut tx, &bundles).await.expect("seed");
    assign_role(&mut tx, with_role.id, ids[0])
        .await
        .expect("assign");
    tx.commit().await.expect("commit");

    let mut tx = Tx::begin(&write, &system_ctx("identity.read"))
        .await
        .expect("read");
    let empty = list_roles_for_principal(&mut tx, lonely.id)
        .await
        .expect("empty");
    assert!(empty.is_empty());

    let roles = list_roles_for_principal(&mut tx, with_role.id)
        .await
        .expect("roles");
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "operator");
    assert!(
        roles[0]
            .permissions
            .contains(&"calibration.view".to_string()),
        "permissions must be populated for principal role list"
    );
    tx.commit().await.expect("commit read");
    db.finish().await.expect("finish");
}
