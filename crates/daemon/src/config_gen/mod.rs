pub mod rpxy;
pub mod rpxy_l4;
pub mod stalwart;

use std::path::{Path, PathBuf};

use crate::state::PersistedState;

pub struct GeneratedPaths {
    pub rpxy: PathBuf,
    pub rpxy_l4: PathBuf,
    pub stalwart: PathBuf,
}

/// Renders all three backend configs from current app state and writes them
/// under `<config_dir>/generated/`. Returns the paths written so the process
/// manager knows what to hand each binary.
pub fn write_all(state: &PersistedState, config_dir: &Path) -> anyhow::Result<GeneratedPaths> {
    let out_dir = config_dir.join("generated");
    std::fs::create_dir_all(&out_dir)?;

    let cert_dir = config_dir.join("certs");
    let cert_dir_str = cert_dir.to_string_lossy().to_string();

    let paths = GeneratedPaths {
        rpxy: out_dir.join("rpxy.toml"),
        rpxy_l4: out_dir.join("rpxy-l4.toml"),
        stalwart: out_dir.join("stalwart.toml"),
    };

    std::fs::write(&paths.rpxy, rpxy::generate(state, &cert_dir_str))?;
    std::fs::write(&paths.rpxy_l4, rpxy_l4::generate(state))?;
    std::fs::write(&paths.stalwart, stalwart::generate(state))?;

    Ok(paths)
}
