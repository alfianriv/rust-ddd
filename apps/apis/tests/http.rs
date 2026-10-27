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
    // Tests in this file run in parallel; migrating concurrently on a fresh DB races
    // (both try to create seaql_migrations). Run it once, the others wait.
    static MIGRATED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    MIGRATED
        .get_or_init(|| async {
            let db = sea_orm::Database::connect(&url).await.unwrap();
            Migrator::up(&db, None).await.unwrap();
        })
        .await;
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

/// Registers + logs in a fresh user. Returns (access token, user id).
async fn signup(app: &Router) -> (String, String) {
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
    (
        r.body["access_token"].as_str().unwrap().to_string(),
        r.body["user"]["id"].as_str().unwrap().to_string(),
    )
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
    let paths = spec["paths"].as_object().unwrap();
    for path in [
        "/auth/register",
        "/auth/login",
        "/users/{id}",
        "/addresses",
        "/addresses/{id}",
        "/addresses/{id}/default",
    ] {
        assert!(paths.contains_key(path), "missing {path} in spec");
    }
    // Conventions: no /me, no PATCH, no nesting one feature under another.
    for (path, ops) in paths {
        assert!(!path.contains("/me"), "no /me routes: {path}");
        assert!(ops.get("patch").is_none(), "no PATCH: {path}");
        assert!(
            !(path.starts_with("/users/") && path.matches('/').count() > 2),
            "no nested routes under /users: {path}"
        );
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
async fn auth_and_user() {
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

    let bad_email = json!({"email": "nope", "name": "A", "password": "secret123"});
    let r = call(&app, Method::POST, "/auth/register", None, Some(bad_email)).await;
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

    let wrong = json!({"email": email, "password": "wrong-pass"});
    let r = call(&app, Method::POST, "/auth/login", None, Some(wrong)).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("unauthorized"))
    );

    let ok = json!({"email": email, "password": "secret123"});
    let r = call(&app, Method::POST, "/auth/login", None, Some(ok)).await;
    assert_eq!(r.body["token_type"], "Bearer");
    assert_eq!(r.body["user"]["id"], registered["id"]);
    let token = r.body["access_token"].as_str().unwrap().to_string();
    let me = format!("/users/{}", registered["id"].as_str().unwrap());

    assert_eq!(
        call(&app, Method::GET, &me, None, None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Method::GET, &me, Some("garbage"), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let r = call(&app, Method::GET, &me, Some(&token), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.body, registered,
        "read-back equals the register response, timestamps included"
    );

    let r = call(
        &app,
        Method::PUT,
        &me,
        Some(&token),
        Some(json!({"name": " Budi "})),
    )
    .await;
    assert_eq!(
        (r.status, r.body["name"].as_str()),
        (StatusCode::OK, Some("Budi"))
    );

    // Ownership: a valid token for someone else's user_id -> 403, nothing changes.
    let (other_token, _) = signup(&app).await;
    let r = call(&app, Method::GET, &me, Some(&other_token), None).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::FORBIDDEN, Some("forbidden"))
    );
    let r = call(
        &app,
        Method::PUT,
        &me,
        Some(&other_token),
        Some(json!({"name": "Hacked"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(&app, Method::GET, &me, Some(&token), None).await;
    assert_eq!(r.body["name"], "Budi");

    let r = call(&app, Method::GET, "/users/not-a-uuid", Some(&token), None).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn shipping_addresses() {
    let Some(app) = app().await else { return };
    let (token, _) = signup(&app).await;
    let t = Some(token.as_str());
    let base = "/addresses";

    assert_eq!(
        call(&app, Method::GET, base, None, None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let a = call(&app, Method::POST, base, t, Some(address("Rumah"))).await;
    assert_eq!(a.status, StatusCode::CREATED, "{}", a.body);
    assert_eq!(
        (a.body["is_default"].as_bool(), a.body["phone"].as_str()),
        (Some(true), Some("081234567890"))
    );
    let a_url = format!("{base}/{}", a.body["id"].as_str().unwrap());

    let mut kantor = address("Kantor");
    kantor["is_default"] = json!(true);
    let b = call(&app, Method::POST, base, t, Some(kantor)).await;
    assert_eq!(b.body["is_default"], true);

    let list = call(&app, Method::GET, base, t, None).await.body;
    let labels: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["Kantor", "Rumah"]);

    let r = call(&app, Method::PUT, &a_url, t, Some(address("Rumah Bogor"))).await;
    assert_eq!(
        (r.status, r.body["label"].as_str()),
        (StatusCode::OK, Some("Rumah Bogor"))
    );

    let mut bad = address("X");
    bad["postal_code"] = json!("123");
    let r = call(&app, Method::POST, base, t, Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);

    let default_url = format!("{a_url}/default");
    for _ in 0..2 {
        // PUT is idempotent: second call is a no-op with the same result.
        let r = call(&app, Method::PUT, &default_url, t, None).await;
        assert_eq!(
            (r.status, r.body["is_default"].as_bool()),
            (StatusCode::OK, Some(true))
        );
    }

    // Ownership comes from the JWT: another user gets 403 on every write, and their
    // list never includes it.
    let (other_token, _) = signup(&app).await;
    let o = Some(other_token.as_str());
    for (method, url, body) in [
        (Method::PUT, a_url.clone(), Some(address("Hacked"))),
        (Method::PUT, default_url.clone(), None),
        (Method::DELETE, a_url.clone(), None),
    ] {
        let r = call(&app, method, &url, o, body).await;
        assert_eq!(
            (r.status, r.body["error"]["code"].as_str()),
            (StatusCode::FORBIDDEN, Some("forbidden")),
            "{url}"
        );
    }
    let theirs = call(&app, Method::GET, base, o, None).await.body;
    assert_eq!(theirs, json!([]));
    let r = call(&app, Method::PUT, &a_url, t, Some(address("Rumah Bogor"))).await;
    assert_eq!(
        r.body["label"], "Rumah Bogor",
        "untouched by the other user"
    );

    let missing = format!("{base}/{}", uuid::Uuid::now_v7());
    let r = call(&app, Method::DELETE, &missing, t, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let r = call(&app, Method::DELETE, &format!("{base}/not-a-uuid"), t, None).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_request"))
    );

    let r = call(&app, Method::DELETE, &a_url, t, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let list = call(&app, Method::GET, base, t, None).await.body;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(
        list[0]["is_default"], true,
        "remaining address promoted to default"
    );
}
