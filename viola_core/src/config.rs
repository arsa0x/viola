use anyhow::{Result, anyhow};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};

use crate::state::AppState;

const DEFAULT_CONFIG: &str = include_str!("../../config.template.toml");

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bot: BotConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BotConfig {
    pub name: String,
    pub prefix: String,
    pub owner: String,
    pub active: bool,
}

pub fn init_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("failed to get home dir"))?;

    let base = home.join("viola");

    fs::create_dir_all(&base)?;

    for dir in ["cache", "downloads"] {
        fs::create_dir_all(base.join(dir))?;
    }

    let config = base.join("config.toml");
    if !config.exists() {
        fs::write(config, DEFAULT_CONFIG)?;
    }

    Ok(base)
}

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub async fn watch_config(state: Arc<AppState>) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.blocking_send(event);
    })
    .unwrap();

    watcher
        .watch(
            &state.dir.join("config.toml").as_path(),
            RecursiveMode::NonRecursive,
        )
        .unwrap();

    while let Some(Ok(_)) = rx.recv().await {
        match load_config(&state.dir.join("config.toml").to_string_lossy()) {
            Ok(cfg) => {
                *state.config.write().await = cfg;
            }
            Err(e) => {
                log::error!("failed reload config: {}", e);
            }
        }
    }
}
