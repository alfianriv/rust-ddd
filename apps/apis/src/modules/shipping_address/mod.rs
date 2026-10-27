mod controller;
mod dto;
mod response;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use controller::*;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_addresses, add_address))
        .routes(routes!(update_address, delete_address))
        .routes(routes!(set_default_address))
}
