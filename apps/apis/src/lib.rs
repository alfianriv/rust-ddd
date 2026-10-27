mod error;
mod extract;
mod modules;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::http::{HeaderName, HeaderValue, Method, Request, StatusCode, header};
use axum::routing::get;
use axum::{Json, Router};
use bootstrap::Container;
use bootstrap::config::HttpConfig;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

pub type AppState = Arc<Container>;

#[derive(OpenApi)]
#[openapi(
    info(title = "apis", description = "Public API. Auth: `Authorization: Bearer <access_token>` from `POST /auth/login`."),
    modifiers(&BearerAuth)
)]
struct ApiDoc;

struct BearerAuth;

impl utoipa::Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi
            .components
            .get_or_insert_default()
            .add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
    }
}

/// Full app: routes, docs (`/docs`, `/openapi.json`), `/health`, middleware.
pub fn router(state: AppState, cfg: &HttpConfig) -> anyhow::Result<Router> {
    let (api, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(modules::user::routes())
        .merge(modules::shipping_address::routes())
        .split_for_parts();

    let spec = openapi.clone();
    let mut app = Router::new()
        .route("/health", get(|| async { Json(health_ok()) }))
        .route("/openapi.json", get(move || async move { Json(spec) }))
        .merge(Scalar::with_url("/docs", openapi))
        .merge(api)
        .with_state(state);
    if let Some(cors) = cors(&cfg.cors_allowed_origins)? {
        app = app.layer(cors);
    }

    let request_id = HeaderName::from_static("x-request-id");
    // Layers run outside-in: last added = first to see the request.
    Ok(app
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            Duration::from_secs(cfg.request_timeout_secs),
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
                let id = req
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default();
                tracing::info_span!("http", method = %req.method(), path = %req.uri().path(), request_id = %id)
            })
            // One access-log line per request: status + latency.
            .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid)))
}

/// Liveness only: the process is up and serving. No DB check.
fn health_ok() -> std::collections::BTreeMap<&'static str, &'static str> {
    [("status", "ok")].into()
}

fn cors(origins: &[String]) -> anyhow::Result<Option<CorsLayer>> {
    if origins.is_empty() {
        return Ok(None);
    }
    let origins = origins
        .iter()
        .map(|o| HeaderValue::from_str(o).with_context(|| format!("invalid CORS origin {o:?}")))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Some(
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
            .expose_headers([HeaderName::from_static("x-request-id")]),
    ))
}
