use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppResult;
use crate::state::MailConfig;
use crate::AppContext;

/// Covers the "Post Office" wizard's declared scope: domain + account list,
/// which feed `config_gen::stalwart`. Account passwords are deliberately not
/// handled here — they should be provisioned through stalwart's own
/// admin tooling once its config schema and directory backend are pinned
/// down, rather than stored in our plaintext state file.
pub fn router() -> Router<AppContext> {
    Router::new().route("/api/mail", get(get_config).put(set_config))
}

async fn get_config(State(ctx): State<AppContext>) -> Json<MailConfig> {
    let guard = ctx.state.read().await;
    Json(guard.persisted.mail.clone())
}

async fn set_config(
    State(ctx): State<AppContext>,
    Json(cfg): Json<MailConfig>,
) -> AppResult<Json<MailConfig>> {
    let mut guard = ctx.state.write().await;
    guard.persisted.mail = cfg.clone();
    guard.save()?;
    Ok(Json(cfg))
}
