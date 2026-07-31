use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppResult;
use crate::state::SecurityConfig;
use crate::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new().route("/api/security", get(get_config).put(set_config))
}

async fn get_config(State(ctx): State<AppContext>) -> Json<SecurityConfig> {
    let guard = ctx.state.read().await;
    Json(guard.persisted.security.clone())
}

async fn set_config(
    State(ctx): State<AppContext>,
    Json(cfg): Json<SecurityConfig>,
) -> AppResult<Json<SecurityConfig>> {
    let mut guard = ctx.state.write().await;
    guard.persisted.security = cfg.clone();
    guard.save()?;
    Ok(Json(cfg))
}
