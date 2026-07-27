use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use whatsapp_rust::anyhow;

use crate::session;

const TEMPLATE_CONFIG: &str = include_str!("../../config.template");

pub const CONFIG_FILE: &str = "config";
pub const DOWNLOAD_DIR: &str = "download";
pub const CACHE_DIR: &str = "cache";

pub fn ensure_config_file(session_dir: &Path) -> anyhow::Result<PathBuf> {
    let path = session_dir.join("config");
    if !path.exists() {
        std::fs::write(&path, TEMPLATE_CONFIG)?;
    }
    Ok(path)
}

pub fn load_for_session(name: &str) -> anyhow::Result<Config> {
    let dir = session::ensure_session_dir(name)?;
    let path = ensure_config_file(&dir)?;
    let text = std::fs::read_to_string(path)?;
    Ok(Config::parse(&text))
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Public,
    Group,
    Owner,
}

impl FromStr for Mode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "public" => Ok(Self::Public),
            "group" => Ok(Self::Group),
            "owner" => Ok(Self::Owner),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub prefixes: Vec<char>,
    pub owners: Vec<String>,
    pub mode: Mode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefixes: vec!['.'],
            owners: Vec::new(),
            mode: Mode::Public,
        }
    }
}

impl Config {
    pub fn load() -> std::io::Result<Self> {
        let content = std::fs::read_to_string(CONFIG_FILE)?;
        Ok(Self::parse(&content))
    }

    pub fn parse(input: &str) -> Self {
        let mut cfg = Self::default();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "prefixes" => {
                    cfg.prefixes = value
                        .split('|')
                        .map(str::trim)
                        .filter(|s| s.chars().count() == 1)
                        .map(|s| s.chars().next().unwrap())
                        .collect();
                }
                "owners" => {
                    cfg.owners = value.split('|').map(|s| s.trim().to_owned()).collect();
                }
                "mode" => {
                    cfg.mode = value.parse().unwrap_or(Mode::Public);
                }
                _ => {}
            }
        }
        cfg
    }
}
