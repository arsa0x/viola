use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use directories::ProjectDirs;
use whatsapp_rust::anyhow;

use crate::session;

const TEMPLATE_CONFIG: &str = include_str!("../../config.template");

#[derive(Debug, Clone)]
pub struct Config {
    pub prefixes: Vec<char>,
    pub owners: Vec<String>,
    pub mode: Mode,
    pub parsed: ParsedConfig,
    pub dirs: ConfigDirs,
}

#[derive(Debug, Clone, Default)]
pub struct ConfigDirs {
    pub sessions: PathBuf,
    pub downloads: PathBuf,
    pub cache: PathBuf,
    pub plugins: PathBuf,
}

impl ConfigDirs {
    pub fn load() -> anyhow::Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "viola")
            .ok_or_else(|| anyhow::anyhow!("failed to determine application directories"))?;

        let config_dir = project_dirs.config_dir();

        Ok(Self {
            sessions: config_dir.join("sessions"),
            downloads: config_dir.join("downloads"),
            cache: config_dir.join("cache"),
            plugins: config_dir.join("plugins"),
        })
    }

    pub fn init() -> anyhow::Result<Self> {
        let dirs = Self::load()?;
        dirs.ensure()?;
        Ok(dirs)
    }

    pub fn ensure(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.sessions)?;
        std::fs::create_dir_all(&self.downloads)?;
        std::fs::create_dir_all(&self.cache)?;
        std::fs::create_dir_all(&self.plugins)?;

        Ok(())
    }
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

    Config::from_str(&content)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Public,
    Group,
    Owner,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Public
    }
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "public" => Ok(Self::Public),
            "group" => Ok(Self::Group),
            "owner" => Ok(Self::Owner),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Default)]
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

            let key = key.trim();

            if key.is_empty() {
                continue;
            }

            values.insert(key.to_owned(), value.trim().to_owned());
        }

        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn get_list(&self, key: &str) -> Vec<String> {
        self.get(key)
            .unwrap_or_default()
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub fn get_chars(&self, key: &str) -> Vec<char> {
        self.get(key)
            .unwrap_or_default()
            .split('|')
            .filter_map(|value| {
                let value = value.trim();

                if value.chars().count() == 1 {
                    value.chars().next()
                } else {
                    None
                }
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
            dirs: ConfigDirs::default(),
        }
    }
}

impl Config {
    pub fn parse(input: &str) -> Self {
        let parsed = ParsedConfig::parse(input);

        Self {
            prefixes: {
                let prefixes = parsed.get_chars("prefixes");

                if prefixes.is_empty() {
                    vec!['.']
                } else {
                    prefixes
                }
            },

            owners: parsed.get_list("owners"),

            mode: parsed
                .get("mode")
                .and_then(|value| value.parse().ok())
                .unwrap_or_default(),

            parsed,

            dirs: ConfigDirs::default(),
        }
    }

    pub fn from_str(input: &str) -> anyhow::Result<Self> {
        let mut config = Self::parse(input);
        config.dirs = ConfigDirs::init()?;

        Ok(config)
    }
}
