mod api;
mod config_gen;
mod dynv6;
mod error;
mod network;
mod process;
mod state;
mod static_server;

use std::sync::Arc;

use process::ProcessManager;
use state::AppState;
use static_server::StaticServerManager;

const LISTEN_ADDR: &str = "127.0.0.1:7878";

#[derive(Clone)]
pub struct AppContext {
    pub state: AppState,
    pub processes: Arc<ProcessManager>,
    pub static_servers: Arc<StaticServerManager>,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = state::init_state()?;
    tracing::info!(path = %state.read().await.state_path.display(), "loaded state");

    let ctx = AppContext {
        state: state.clone(),
        processes: ProcessManager::new(),
        static_servers: StaticServerManager::new(),
        http_client: reqwest::Client::new(),
    };

    tokio::spawn(dynv6::run_sync_loop(state));

    let app = api::build_router(ctx);
    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR).await?;
    tracing::info!("pocketserver-daemon listening on http://{LISTEN_ADDR}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
