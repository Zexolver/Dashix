use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppResult;
use crate::network::scan_interfaces;
use crate::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new().route("/api/network/interfaces", get(list_interfaces))
}

async fn list_interfaces() -> AppResult<Json<serde_json::Value>> {
    let interfaces = scan_interfaces()?;
    Ok(Json(serde_json::json!({ "interfaces": interfaces })))
}
