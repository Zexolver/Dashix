use toml::map::Map;
use toml::Value;

use crate::state::PersistedState;

/// Generates a starting-point stalwart-mail TOML config for the configured
/// domain and accounts. Stalwart's schema is large and has changed across
/// releases (storage backends, directory types, TLS/ACME, listeners); this
/// covers only the "Post Office" wizard's declared scope (domain + account
/// list) and should be validated/extended against the installed stalwart
/// version rather than trusted as a complete config.
pub fn generate(state: &PersistedState) -> String {
    let mut doc = Map::new();

    if let Some(domain) = &state.mail.domain {
        let mut server = Map::new();
        server.insert("hostname".into(), Value::String(domain.clone()));
        doc.insert("server".into(), Value::Table(server));
    }

    let mut internal_dir = Map::new();
    internal_dir.insert("type".into(), Value::String("internal".into()));
    let principals: Vec<Value> = state
        .mail
        .accounts
        .iter()
        .map(|acct| {
            let mut p = Map::new();
            p.insert("type".into(), Value::String("individual".into()));
            p.insert("name".into(), Value::String(acct.address.clone()));
            p.insert(
                "description".into(),
                Value::String(acct.display_name.clone()),
            );
            Value::Table(p)
        })
        .collect();
    if !principals.is_empty() {
        internal_dir.insert("principals".into(), Value::Array(principals));
    }

    let mut directory = Map::new();
    directory.insert("internal".into(), Value::Table(internal_dir));
    doc.insert("directory".into(), Value::Table(directory));

    toml::to_string_pretty(&Value::Table(doc)).unwrap_or_default()
}
