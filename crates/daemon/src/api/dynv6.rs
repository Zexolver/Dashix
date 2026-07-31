use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::dynv6::sync_now;
use crate::error::AppResult;
use crate::state::Dynv6Config;
use crate::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/api/dynv6", get(get_config).put(set_config))
        .route("/api/dynv6/status", get(get_status))
        .route("/api/dynv6/sync", post(trigger_sync))
}

async fn get_config(State(ctx): State<AppContext>) -> Json<Dynv6Config> {
    let guard = ctx.state.read().await;
    Json(guard.persisted.dynv6.clone())
}

async fn set_config(
    State(ctx): State<AppContext>,
    Json(cfg): Json<Dynv6Config>,
) -> AppResult<Json<Dynv6Config>> {
    let mut guard = ctx.state.write().await;
    guard.persisted.dynv6 = cfg.clone();
    guard.save()?;
    Ok(Json(cfg))
}

async fn get_status(State(ctx): State<AppContext>) -> Json<serde_json::Value> {
    let guard = ctx.state.read().await;
    Json(serde_json::json!({ "status": guard.dynv6_status }))
}

async fn trigger_sync(State(ctx): State<AppContext>) -> Json<serde_json::Value> {
    let statuses = sync_now(&ctx.state, &ctx.http_client).await;
    Json(serde_json::json!({ "status": statuses }))
}
