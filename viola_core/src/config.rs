use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use directories::ProjectDirs;
use whatsapp_rust::anyhow;

use crate::session;

const TEMPLATE_CONFIG: &str = include_str!("../../config.template");

#[derive(Debug)]
pub struct Config {
    pub prefixes: Vec<char>,
    pub owners: Vec<String>,
    pub mode: Mode,
    pub parsed: ParsedConfig,
}

#[derive(Debug)]
pub struct ConfigDirs {
    pub sessions: PathBuf,
    pub downloads: PathBuf,
    pub cache: PathBuf,
}

impl ConfigDirs {
    fn load() -> anyhow::Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "viola")
            .ok_or_else(|| anyhow::anyhow!("failed to determine application directory"))?;

        let config_dir = project_dirs.config_dir();

        Ok(Self {
            sessions: config_dir.join("sessions"),
            downloads: config_dir.join("downloads"),
            cache: config_dir.join("cache"),
        })
    }

    pub fn ensure(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.sessions)?;
        std::fs::create_dir_all(&self.downloads)?;
        std::fs::create_dir_all(&self.cache)?;
        Ok(())
    }
}
pub fn init() -> anyhow::Result<ConfigDirs> {
    let dirs = ConfigDirs::load()?;
    dirs.ensure()?;

    Ok(dirs)
}

pub fn ensure_config_file(session_dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(session_dir)?;

    let path = session_dir.join("config");

    if !path.exists() {
        std::fs::write(&path, TEMPLATE_CONFIG)?;
    }

    Ok(path)
}

pub fn load_for_session(name: &str) -> anyhow::Result<Config> {
    let session_dir = session::ensure_session_dir(name)?;
    let config_path = ensure_config_file(&session_dir)?;

    let content = std::fs::read_to_string(config_path)?;

    Ok(Config::parse(&content))
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

#[derive(Debug, Default)]
pub struct ParsedConfig {
    values: HashMap<String, String>,
}

impl ParsedConfig {
    pub fn parse(input: &str) -> Self {
        let mut values = HashMap::new();

        for line in input.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            values.insert(key.trim().to_owned(), value.trim().to_owned());
        }

        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn get_list(&self, key: &str) -> Vec<String> {
        self.get(key)
            .unwrap_or("")
            .split('|')
            .map(|s| s.trim().to_owned())
            .collect()
    }

    pub fn get_chars(&self, key: &str) -> Vec<char> {
        self.get(key)
            .unwrap_or("")
            .split('|')
            .filter_map(|s| {
                let s = s.trim();
                (s.chars().count() == 1).then(|| s.chars().next().unwrap())
            })
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefixes: vec!['.'],
            owners: Vec::new(),
            mode: Mode::Public,
            parsed: ParsedConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> std::io::Result<Self> {
        let content = std::fs::read_to_string("config")?;
        Ok(Self::parse(&content))
    }

    pub fn parse(input: &str) -> Self {
        let parsed = ParsedConfig::parse(input);

        Self {
            prefixes: parsed.get_chars("prefixes"),
            owners: parsed.get_list("owners"),
            mode: parsed
                .get("mode")
                .and_then(|s| s.parse().ok())
                .unwrap_or(Mode::Public),
            parsed,
        }
    }
}
