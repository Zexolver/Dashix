use toml::map::Map;
use toml::Value;

use crate::state::{L4Protocol, PersistedState};

/// Generates rpxy-l4's TOML config from the Security Shield's raw TCP/UDP
/// rules (Minecraft, SSH, RustDesk, etc). As with rpxy, validate this against
/// the exact rpxy-l4 version deployed.
pub fn generate(state: &PersistedState) -> String {
    let mut tcp_rules = Vec::new();
    let mut udp_rules = Vec::new();

    for rule in &state.security.l4_rules {
        let mut t = Map::new();
        t.insert("name".into(), Value::String(rule.name.clone()));
        t.insert(
            "listen_port".into(),
            Value::Integer(rule.listen_port as i64),
        );
        t.insert(
            "upstream".into(),
            Value::String(format!("127.0.0.1:{}", rule.upstream_port)),
        );
        match rule.protocol {
            L4Protocol::Tcp => tcp_rules.push(Value::Table(t)),
            L4Protocol::Udp => udp_rules.push(Value::Table(t)),
        }
    }

    let mut doc = Map::new();
    if !tcp_rules.is_empty() {
        doc.insert("tcp".into(), Value::Array(tcp_rules));
    }
    if !udp_rules.is_empty() {
        doc.insert("udp".into(), Value::Array(udp_rules));
    }
    if !state.security.blocked_ips.is_empty() {
        doc.insert(
            "blocked_ips".into(),
            Value::Array(
                state
                    .security
                    .blocked_ips
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }

    toml::to_string_pretty(&Value::Table(doc)).unwrap_or_default()
}
