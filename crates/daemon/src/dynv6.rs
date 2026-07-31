use std::time::Duration;

use tracing::{info, warn};

use crate::network::primary_addrs;
use crate::state::{AppState, Dynv6SyncStatus};

const SYNC_INTERVAL: Duration = Duration::from_secs(300);

/// Background task: periodically pushes the current interface's IP to every
/// configured dynv6 hostname. Runs for the lifetime of the daemon.
pub async fn run_sync_loop(state: AppState) {
    let client = reqwest::Client::new();
    loop {
        sync_now(&state, &client).await;
        tokio::time::sleep(SYNC_INTERVAL).await;
    }
}

/// Runs one sync pass immediately and stores the resulting status, returning
/// it too so the API can respond to an explicit "sync now" request.
pub async fn sync_now(state: &AppState, client: &reqwest::Client) -> Vec<Dynv6SyncStatus> {
    let (enabled, token, interface, domains) = {
        let guard = state.read().await;
        let cfg = &guard.persisted.dynv6;
        (
            cfg.enabled,
            cfg.token.clone(),
            cfg.interface.clone(),
            cfg.domains.clone(),
        )
    };

    if !enabled || domains.is_empty() {
        return Vec::new();
    }
    let Some(token) = token else {
        warn!("dynv6 sync enabled but no token configured");
        return Vec::new();
    };
    let Some(interface) = interface else {
        warn!("dynv6 sync enabled but no interface selected");
        return Vec::new();
    };

    let (ipv4, ipv6) = match primary_addrs(&interface) {
        Ok(addrs) => addrs,
        Err(e) => {
            warn!("dynv6 sync: failed to read addresses for {interface}: {e}");
            return Vec::new();
        }
    };

    let mut statuses = Vec::with_capacity(domains.len());
    for domain in &domains {
        statuses.push(sync_domain(client, &token, domain, ipv4.as_deref(), ipv6.as_deref()).await);
    }

    let mut guard = state.write().await;
    guard.dynv6_status = statuses.clone();
    statuses
}

async fn sync_domain(
    client: &reqwest::Client,
    token: &str,
    hostname: &str,
    ipv4: Option<&str>,
    ipv6: Option<&str>,
) -> Dynv6SyncStatus {
    let now = now_epoch_secs();
    let mut url = format!(
        "https://dynv6.com/api/update?hostname={}&token={}",
        urlencode(hostname),
        urlencode(token)
    );
    if let Some(ip) = ipv4 {
        url.push_str(&format!("&ipv4={}", urlencode(ip)));
    }
    if let Some(ip) = ipv6 {
        url.push_str(&format!("&ipv6={}", urlencode(ip)));
    }

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!("dynv6: synced {hostname}");
            Dynv6SyncStatus {
                domain: hostname.to_string(),
                last_attempt: Some(now.clone()),
                last_success: Some(now),
                last_error: None,
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("dynv6: update for {hostname} failed: {status} {body}");
            Dynv6SyncStatus {
                domain: hostname.to_string(),
                last_attempt: Some(now),
                last_success: None,
                last_error: Some(format!("{status}: {body}")),
            }
        }
        Err(e) => {
            warn!("dynv6: request for {hostname} failed: {e}");
            Dynv6SyncStatus {
                domain: hostname.to_string(),
                last_attempt: Some(now),
                last_success: None,
                last_error: Some(e.to_string()),
            }
        }
    }
}

/// Minimal percent-encoding, sufficient for tokens/hostnames/IP literals in a
/// query string (avoids pulling in a full URL-encoding crate for this).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn now_epoch_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
