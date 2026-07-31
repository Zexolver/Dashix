pub mod dynv6;
pub mod mail;
pub mod network;
pub mod routes;
pub mod security;
pub mod services;

use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppContext;

pub fn build_router(ctx: AppContext) -> Router {
    Router::new()
        .route("/api/status", get(status))
        .merge(network::router())
        .merge(dynv6::router())
        .merge(routes::router())
        .merge(services::router())
        .merge(mail::router())
        .merge(security::router())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(ctx)
}

async fn status() -> &'static str {
    "ok"
}
