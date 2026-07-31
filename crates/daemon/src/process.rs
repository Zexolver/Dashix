use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Rpxy,
    RpxyL4,
    Stalwart,
}

const ALL_KINDS: [ServiceKind; 3] = [
    ServiceKind::Rpxy,
    ServiceKind::RpxyL4,
    ServiceKind::Stalwart,
];

impl ServiceKind {
    fn binary_name(&self) -> &'static str {
        match self {
            ServiceKind::Rpxy => "rpxy",
            ServiceKind::RpxyL4 => "rpxy-l4",
            ServiceKind::Stalwart => "stalwart",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub kind: ServiceKind,
    pub running: bool,
    pub pid: Option<u32>,
}

/// Manages the three wrapped binaries as child processes: starts them with
/// their generated config, tracks liveness, and lets the API restart them
/// after config changes ("apply").
pub struct ProcessManager {
    children: Mutex<HashMap<ServiceKind, Child>>,
}

impl ProcessManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            children: Mutex::new(HashMap::new()),
        })
    }

    pub async fn start(&self, kind: ServiceKind, config_path: &Path) -> anyhow::Result<()> {
        let mut children = self.children.lock().await;
        if let Some(child) = children.get_mut(&kind) {
            if matches!(child.try_wait(), Ok(None)) {
                anyhow::bail!("{:?} is already running", kind);
            }
        }

        let child = Command::new(kind.binary_name())
            .arg("--config")
            .arg(config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to spawn {}: {e} (is it installed and on PATH?)",
                    kind.binary_name()
                )
            })?;

        children.insert(kind, child);
        Ok(())
    }

    pub async fn stop(&self, kind: ServiceKind) -> anyhow::Result<()> {
        let mut children = self.children.lock().await;
        if let Some(mut child) = children.remove(&kind) {
            child.kill().await.ok();
        }
        Ok(())
    }

    pub async fn restart(&self, kind: ServiceKind, config_path: &Path) -> anyhow::Result<()> {
        self.stop(kind).await?;
        self.start(kind, config_path).await
    }

    pub async fn status(&self) -> Vec<ServiceStatus> {
        let mut children = self.children.lock().await;
        let mut out = Vec::with_capacity(ALL_KINDS.len());
        for kind in ALL_KINDS {
            let (running, pid) = match children.get_mut(&kind) {
                Some(child) => match child.try_wait() {
                    Ok(None) => (true, child.id()),
                    _ => (false, None),
                },
                None => (false, None),
            };
            out.push(ServiceStatus { kind, running, pid });
        }
        out
    }
}
