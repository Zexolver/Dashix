use axum::extract::{Path as AxumPath, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::error::{AppError, AppResult};
use crate::process::ServiceKind;
use crate::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/api/services", get(status))
        .route("/api/services/:name/:action", post(control))
}

async fn status(State(ctx): State<AppContext>) -> Json<serde_json::Value> {
    let statuses = ctx.processes.status().await;
    Json(serde_json::json!({ "services": statuses }))
}

fn parse_kind(name: &str) -> AppResult<ServiceKind> {
    match name {
        "rpxy" => Ok(ServiceKind::Rpxy),
        "rpxy-l4" | "rpxy_l4" => Ok(ServiceKind::RpxyL4),
        "stalwart" => Ok(ServiceKind::Stalwart),
        other => Err(AppError::BadRequest(format!("unknown service {other}"))),
    }
}

async fn control(
    State(ctx): State<AppContext>,
    AxumPath((name, action)): AxumPath<(String, String)>,
) -> AppResult<Json<serde_json::Value>> {
    let kind = parse_kind(&name)?;

    match action.as_str() {
        "stop" => {
            ctx.processes.stop(kind).await?;
        }
        "start" | "restart" => {
            let (persisted, config_dir) = {
                let guard = ctx.state.read().await;
                (guard.persisted.clone(), guard.config_dir.clone())
            };
            let paths = crate::config_gen::write_all(&persisted, &config_dir)?;
            let path = match kind {
                ServiceKind::Rpxy => &paths.rpxy,
                ServiceKind::RpxyL4 => &paths.rpxy_l4,
                ServiceKind::Stalwart => &paths.stalwart,
            };
            if action == "start" {
                ctx.processes.start(kind, path).await?;
            } else {
                ctx.processes.restart(kind, path).await?;
            }
        }
        other => return Err(AppError::BadRequest(format!("unknown action {other}"))),
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
