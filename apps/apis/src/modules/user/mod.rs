mod controller;
mod dto;
mod response;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use controller::*;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login))
        .routes(routes!(get_me, update_me))
}
