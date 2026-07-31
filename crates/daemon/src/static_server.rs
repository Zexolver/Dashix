use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tower_http::services::ServeDir;
use uuid::Uuid;

/// rpxy is a reverse proxy, not a file server, so "point a subdomain at a
/// static folder" is implemented by having the daemon itself serve that
/// folder on a local port and having rpxy reverse-proxy to it. `ServeDir`
/// reads from disk on every request, so hot reload is free — there's no build
/// step to trigger.
#[derive(Default)]
pub struct StaticServerManager {
    servers: Mutex<HashMap<Uuid, JoinHandle<()>>>,
}

impl StaticServerManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Starts serving `path` on `port` for `route_id` if it isn't already
    /// running. If the route's folder path changes, callers should `stop`
    /// then `ensure_running` again to pick up the new root.
    pub async fn ensure_running(&self, route_id: Uuid, port: u16, path: String) {
        let mut servers = self.servers.lock().await;
        if servers.contains_key(&route_id) {
            return;
        }
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let service = ServeDir::new(path);
        let app = axum::Router::new().fallback_service(service);
        let handle = tokio::spawn(async move {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    if let Err(e) = axum::serve(listener, app).await {
                        tracing::warn!("static server on {addr} exited: {e}");
                    }
                }
                Err(e) => tracing::warn!("static server failed to bind {addr}: {e}"),
            }
        });
        servers.insert(route_id, handle);
    }

    pub async fn stop(&self, route_id: Uuid) {
        let mut servers = self.servers.lock().await;
        if let Some(handle) = servers.remove(&route_id) {
            handle.abort();
        }
    }

    /// Stops any running static servers for routes that no longer exist.
    pub async fn retain(&self, keep: &[Uuid]) {
        let mut servers = self.servers.lock().await;
        let to_remove: Vec<Uuid> = servers
            .keys()
            .filter(|id| !keep.contains(id))
            .copied()
            .collect();
        for id in to_remove {
            if let Some(handle) = servers.remove(&id) {
                handle.abort();
            }
        }
    }
}

/// Lets the OS pick an unused loopback port (bind-to-0 trick), for auto
/// wiring static routes without the user ever choosing a port number.
pub fn pick_free_port() -> anyhow::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
