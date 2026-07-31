use axum::extract::{Path as AxumPath, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::config_gen;
use crate::error::{AppError, AppResult};
use crate::process::ServiceKind;
use crate::state::{RouteEntry, RouteTarget};
use crate::static_server::pick_free_port;
use crate::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/api/routes", get(list_routes).post(create_route))
        .route("/api/routes/:id", delete(delete_route))
        .route("/api/apply", post(apply))
}

#[derive(Deserialize)]
struct NewRoute {
    subdomain: String,
    target: RouteTarget,
    tls: bool,
}

async fn list_routes(State(ctx): State<AppContext>) -> Json<Vec<RouteEntry>> {
    let guard = ctx.state.read().await;
    Json(guard.persisted.routes.clone())
}

async fn create_route(
    State(ctx): State<AppContext>,
    Json(req): Json<NewRoute>,
) -> AppResult<Json<RouteEntry>> {
    let internal_port = match &req.target {
        RouteTarget::Static { .. } => Some(pick_free_port()?),
        RouteTarget::Port { .. } => None,
    };

    let entry = RouteEntry {
        id: Uuid::new_v4(),
        subdomain: req.subdomain,
        target: req.target,
        tls: req.tls,
        internal_port,
    };

    let mut guard = ctx.state.write().await;
    guard.persisted.routes.push(entry.clone());
    guard.save()?;
    Ok(Json(entry))
}

async fn delete_route(
    State(ctx): State<AppContext>,
    AxumPath(id): AxumPath<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let mut guard = ctx.state.write().await;
    let before = guard.persisted.routes.len();
    guard.persisted.routes.retain(|r| r.id != id);
    if guard.persisted.routes.len() == before {
        return Err(AppError::NotFound(format!("route {id}")));
    }
    guard.save()?;
    drop(guard);
    ctx.static_servers.stop(id).await;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

/// Regenerates rpxy/rpxy-l4/stalwart configs from current state, makes sure
/// any static-folder routes have their local file server running, and
/// restarts the three backend services to pick them up. Per-service start
/// failures (e.g. binary not installed / not on PATH) are reported back
/// rather than failing the whole request, since config generation itself
/// always succeeds independent of whether the wrapped binaries exist yet.
async fn apply(State(ctx): State<AppContext>) -> AppResult<Json<serde_json::Value>> {
    let (persisted, config_dir) = {
        let guard = ctx.state.read().await;
        (guard.persisted.clone(), guard.config_dir.clone())
    };

    let mut live_ids = Vec::with_capacity(persisted.routes.len());
    for route in &persisted.routes {
        if let RouteTarget::Static { path, .. } = &route.target {
            if let Some(port) = route.internal_port {
                ctx.static_servers
                    .ensure_running(route.id, port, path.clone())
                    .await;
            }
        }
        live_ids.push(route.id);
    }
    ctx.static_servers.retain(&live_ids).await;

    let paths = config_gen::write_all(&persisted, &config_dir)?;

    let mut service_results = serde_json::Map::new();
    for (name, kind, path) in [
        ("rpxy", ServiceKind::Rpxy, &paths.rpxy),
        ("rpxy_l4", ServiceKind::RpxyL4, &paths.rpxy_l4),
        ("stalwart", ServiceKind::Stalwart, &paths.stalwart),
    ] {
        let result = ctx.processes.restart(kind, path).await;
        service_results.insert(
            name.to_string(),
            match result {
                Ok(()) => serde_json::json!({ "ok": true }),
                Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
            },
        );
    }

    Ok(Json(
        serde_json::json!({ "applied": true, "services": service_results }),
    ))
}
