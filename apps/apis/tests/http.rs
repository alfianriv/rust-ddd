//! Full HTTP stack (router + middleware + real Postgres/argon2/JWT), no network socket.
//! Same skip rule as the other DB tests: needs TEST_DATABASE_URL, required in CI.
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use bootstrap::config::{DatabaseConfig, HttpConfig, PasswordHasherConfig, TokenConfig};
use bootstrap::{Config, Container};
use migration::{Migrator, MigratorTrait};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> Option<Router> {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_DATABASE_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_DATABASE_URL to run DB tests");
        return None;
    };
    let db = sea_orm::Database::connect(&url).await.unwrap();
    Migrator::up(&db, None).await.unwrap();
    let cfg = Config {
        database: DatabaseConfig::Postgres { url },
        password_hasher: PasswordHasherConfig::Argon2,
        token: TokenConfig::Jwt {
            secret: "test-secret-test-secret-test-secret".into(),
            ttl_secs: 60,
        },
        apis: HttpConfig::default(),
    };
    let container = Arc::new(Container::new(&cfg).await.unwrap());
    Some(apis::router(container, &cfg.apis).unwrap())
}

struct Res {
    status: StatusCode,
    request_id: bool,
    body: Value,
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Res {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let request_id = res.headers().contains_key("x-request-id");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res {
        status,
        request_id,
        body,
    }
}

fn unique_email() -> String {
    format!("{}@test.local", uuid::Uuid::now_v7().simple())
}

/// Registers + logs in a fresh user, returns the access token.
async fn signup(app: &Router) -> String {
    let email = unique_email();
    let r = call(
        app,
        Method::POST,
        "/auth/register",
        None,
        Some(json!({"email": email, "name": "Alfian", "password": "secret123"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = call(
        app,
        Method::POST,
        "/auth/login",
        None,
        Some(json!({"email": email, "password": "secret123"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    r.body["access_token"].as_str().unwrap().to_string()
}

fn address(label: &str) -> Value {
    json!({
        "label": label, "recipient_name": "Alfian", "phone": "0812-3456-7890",
        "address_line": "Jl. Pajajaran No. 1", "city": "Bogor",
        "province": "Jawa Barat", "postal_code": "16143"
    })
}

#[tokio::test]
async fn health_docs_and_request_id() {
    let Some(app) = app().await else { return };
    let r = call(&app, Method::GET, "/health", None, None).await;
    assert_eq!(
        (r.status, r.body),
        (StatusCode::OK, json!({"status": "ok"}))
    );
    assert!(r.request_id, "x-request-id header set on responses");

    let spec = call(&app, Method::GET, "/openapi.json", None, None)
        .await
        .body;
    for path in [
        "/auth/register",
        "/auth/login",
        "/me",
        "/me/addresses",
        "/me/addresses/{id}",
        "/me/addresses/{id}/default",
    ] {
        assert!(spec["paths"][path].is_object(), "missing {path} in spec");
    }
    assert_eq!(
        spec["components"]["securitySchemes"]["bearer"]["scheme"],
        "bearer"
    );
    assert_eq!(
        call(&app, Method::GET, "/docs", None, None).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn auth_and_profile() {
    let Some(app) = app().await else { return };
    let email = unique_email();
    let reg = json!({"email": email, "name": "Alfian", "password": "secret123"});

    let r = call(
        &app,
        Method::POST,
        "/auth/register",
        None,
        Some(reg.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert!(
        r.body.get("password_hash").is_none(),
        "hash never leaves the API"
    );
    let registered = r.body;

    let r = call(&app, Method::POST, "/auth/register", None, Some(reg)).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("conflict"))
    );

    let r = call(
        &app,
        Method::POST,
        "/auth/register",
        None,
        Some(json!({"email": "nope", "name": "A", "password": "secret123"})),
    )
    .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("validation_error"))
    );

    let r = call(
        &app,
        Method::POST,
        "/auth/register",
        None,
        Some(json!({"email": email})),
    )
    .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_request"))
    );

    let r = call(
        &app,
        Method::POST,
        "/auth/login",
        None,
        Some(json!({"email": email, "password": "wrong-pass"})),
    )
    .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("unauthorized"))
    );

    let r = call(
        &app,
        Method::POST,
        "/auth/login",
        None,
        Some(json!({"email": email, "password": "secret123"})),
    )
    .await;
    assert_eq!(r.body["token_type"], "Bearer");
    let token = r.body["access_token"].as_str().unwrap().to_string();

    assert_eq!(
        call(&app, Method::GET, "/me", None, None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Method::GET, "/me", Some("garbage"), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let r = call(&app, Method::GET, "/me", Some(&token), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.body, registered,
        "read-back equals the register response, timestamps included"
    );

    let r = call(
        &app,
        Method::PATCH,
        "/me",
        Some(&token),
        Some(json!({"name": " Budi "})),
    )
    .await;
    assert_eq!(
        (r.status, r.body["name"].as_str()),
        (StatusCode::OK, Some("Budi"))
    );
}

#[tokio::test]
async fn shipping_addresses() {
    let Some(app) = app().await else { return };
    let token = signup(&app).await;
    let t = Some(token.as_str());

    let a = call(
        &app,
        Method::POST,
        "/me/addresses",
        t,
        Some(address("Rumah")),
    )
    .await;
    assert_eq!(a.status, StatusCode::CREATED, "{}", a.body);
    assert_eq!(
        (a.body["is_default"].as_bool(), a.body["phone"].as_str()),
        (Some(true), Some("081234567890"))
    );
    let a_id = a.body["id"].as_str().unwrap().to_string();

    let mut kantor = address("Kantor");
    kantor["is_default"] = json!(true);
    let b = call(&app, Method::POST, "/me/addresses", t, Some(kantor)).await;
    assert_eq!(b.body["is_default"], true);

    let list = call(&app, Method::GET, "/me/addresses", t, None).await.body;
    let labels: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["Kantor", "Rumah"]);

    let r = call(
        &app,
        Method::PUT,
        &format!("/me/addresses/{a_id}"),
        t,
        Some(address("Rumah Bogor")),
    )
    .await;
    assert_eq!(
        (r.status, r.body["label"].as_str()),
        (StatusCode::OK, Some("Rumah Bogor"))
    );

    let mut bad = address("X");
    bad["postal_code"] = json!("123");
    let r = call(&app, Method::POST, "/me/addresses", t, Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);

    let r = call(
        &app,
        Method::POST,
        &format!("/me/addresses/{a_id}/default"),
        t,
        None,
    )
    .await;
    assert_eq!(
        (r.status, r.body["is_default"].as_bool()),
        (StatusCode::OK, Some(true))
    );

    // Another user sees 404, not 403: existence of others' data is not revealed.
    let other = signup(&app).await;
    let r = call(
        &app,
        Method::DELETE,
        &format!("/me/addresses/{a_id}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let r = call(&app, Method::DELETE, "/me/addresses/not-a-uuid", t, None).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_request"))
    );

    let r = call(
        &app,
        Method::DELETE,
        &format!("/me/addresses/{a_id}"),
        t,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let list = call(&app, Method::GET, "/me/addresses", t, None).await.body;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(
        list[0]["is_default"], true,
        "remaining address promoted to default"
    );
}
