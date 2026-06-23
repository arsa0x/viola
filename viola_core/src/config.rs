use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const DEFAULT_CONFIG: &str = include_str!("../../config.template.toml");

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bot: BotConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BotConfig {
    pub name: String,
    pub prefix: String,
    pub owners: Vec<String>,
    pub mode: BotMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BotMode {
    Public,
    Group,
    Owner,
    Disabled,
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
