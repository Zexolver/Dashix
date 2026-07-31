use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Where a subdomain's traffic actually goes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteTarget {
    /// Serve a local folder. The daemon runs its own static file server for this
    /// (see `static_server`) and rpxy reverse-proxies to it, since rpxy itself
    /// only speaks reverse-proxy, not static files.
    Static { path: String, hot_reload: bool },
    /// Reverse-proxy straight to a local TCP port (e.g. a Docker container).
    Port { port: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub id: Uuid,
    pub subdomain: String,
    pub target: RouteTarget,
    pub tls: bool,
    /// Assigned automatically when `target` is `Static`: the local port the
    /// built-in static file server listens on for this route.
    pub internal_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dynv6Config {
    pub enabled: bool,
    pub token: Option<String>,
    /// Name of the network interface whose address should be synced (e.g. "eth0").
    pub interface: Option<String>,
    /// Dynv6 hostnames/zones to keep updated, e.g. "myzone.dynv6.net".
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum L4Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L4Rule {
    pub id: Uuid,
    pub name: String,
    pub listen_port: u16,
    pub protocol: L4Protocol,
    pub upstream_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    pub l4_rules: Vec<L4Rule>,
    pub blocked_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAccount {
    pub address: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MailConfig {
    pub domain: Option<String>,
    pub accounts: Vec<MailAccount>,
}

/// Everything that gets written to disk. Secrets that belong to the wrapped
/// services (e.g. mail account passwords) are deliberately NOT part of this —
/// they're handed to stalwart directly at apply-time instead of being kept in
/// our own plaintext JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub dynv6: Dynv6Config,
    pub routes: Vec<RouteEntry>,
    pub security: SecurityConfig,
    pub mail: MailConfig,
}

impl PersistedState {
    pub fn load_or_default(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading state file {}", path.display()))?;
        let state = serde_json::from_str(&raw)
            .with_context(|| format!("parsing state file {}", path.display()))?;
        Ok(state)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating state dir {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw)
            .with_context(|| format!("writing state file {}", path.display()))?;
        Ok(())
    }
}

/// Runtime-only status info, not persisted: last dynv6 sync result per domain.
#[derive(Debug, Clone, Serialize)]
pub struct Dynv6SyncStatus {
    pub domain: String,
    pub last_attempt: Option<String>,
    pub last_success: Option<String>,
    pub last_error: Option<String>,
}

pub struct AppStateInner {
    pub persisted: PersistedState,
    pub dynv6_status: Vec<Dynv6SyncStatus>,
    pub state_path: PathBuf,
    pub config_dir: PathBuf,
}

pub type AppState = Arc<RwLock<AppStateInner>>;

/// `directories::ProjectDirs` falls back to XDG's `$HOME/.config` when it
/// can't otherwise resolve a config dir -- fine on desktop Linux, but
/// Android apps have no conventional `$HOME`, so this silently resolved to
/// `/.config/pocketserver` (unwritable: `/` is read-only, even for root)
/// and every state-saving request 500'd. On Android, the UI passes its
/// own writable private data directory through this env var when it spawns
/// the bundled daemon (see crates/ui/src/android_priv.rs); desktop usage
/// is unaffected since nothing sets it there.
pub fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("POCKETSERVER_STATE_DIR") {
        return PathBuf::from(dir);
    }
    directories::ProjectDirs::from("com", "zexolver", "pocketserver")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".pocketserver"))
}

pub fn init_state() -> anyhow::Result<AppState> {
    let config_dir = state_dir();
    let state_path = config_dir.join("state.json");
    let persisted = PersistedState::load_or_default(&state_path)?;
    Ok(Arc::new(RwLock::new(AppStateInner {
        persisted,
        dynv6_status: Vec::new(),
        state_path,
        config_dir,
    })))
}

impl AppStateInner {
    pub fn save(&self) -> anyhow::Result<()> {
        self.persisted.save(&self.state_path)
    }
}
