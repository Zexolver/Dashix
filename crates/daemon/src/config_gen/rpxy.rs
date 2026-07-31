use toml::map::Map;
use toml::Value;

use crate::state::{PersistedState, RouteTarget};

/// Generates rpxy's TOML config from the current App Router routes. Schema
/// follows rpxy's public docs (https://github.com/junkurihara/rust-rpxy) —
/// double-check against the exact rpxy version being deployed, since its
/// config format has shifted across releases.
pub fn generate(state: &PersistedState, cert_dir: &str) -> String {
    let mut doc = Map::new();
    doc.insert("listen_port".into(), Value::Integer(80));
    doc.insert("listen_port_tls".into(), Value::Integer(443));

    let mut apps = Map::new();
    for route in &state.routes {
        let upstream_port = match &route.target {
            RouteTarget::Port { port } => Some(*port),
            RouteTarget::Static { .. } => route.internal_port,
        };
        let Some(upstream_port) = upstream_port else {
            // Static route with no internal static-server port assigned yet.
            continue;
        };

        let mut app = Map::new();
        app.insert("server_name".into(), Value::String(route.subdomain.clone()));

        let mut upstream_entry = Map::new();
        upstream_entry.insert(
            "location".into(),
            Value::String(format!("127.0.0.1:{upstream_port}")),
        );
        let mut reverse_proxy_entry = Map::new();
        reverse_proxy_entry.insert(
            "upstream".into(),
            Value::Array(vec![Value::Table(upstream_entry)]),
        );
        app.insert(
            "reverse_proxy".into(),
            Value::Array(vec![Value::Table(reverse_proxy_entry)]),
        );

        if route.tls {
            let mut tls = Map::new();
            tls.insert(
                "tls_cert_path".into(),
                Value::String(format!("{cert_dir}/{}/fullchain.pem", route.subdomain)),
            );
            tls.insert(
                "tls_cert_key_path".into(),
                Value::String(format!("{cert_dir}/{}/privkey.pem", route.subdomain)),
            );
            tls.insert("https_redirection".into(), Value::Boolean(true));
            tls.insert("acme".into(), Value::Boolean(true));
            app.insert("tls".into(), Value::Table(tls));
        }

        apps.insert(route.subdomain.clone(), Value::Table(app));
    }
    doc.insert("apps".into(), Value::Table(apps));

    toml::to_string_pretty(&Value::Table(doc)).unwrap_or_default()
}
