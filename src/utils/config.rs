use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::{fs, path::PathBuf};

const DEFAULT_CONFIG: &str = "[bot]\nname = \"viola\"\nprefix = \".\"\nowner = \"\"\n";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
}

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    pub name: String,
    pub prefix: String,
    pub owner: String,
}

pub fn init_dir() -> Result<PathBuf> {
    let data = dirs::data_dir().ok_or_else(|| anyhow!("failed to get data dir"))?;

    let base = data.join("viola");

    fs::create_dir_all(&base)?;

    for dir in ["cache", "store", "downloads"] {
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
