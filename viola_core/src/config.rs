use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use whatsapp_rust::anyhow;

use crate::session::Session;

const TEMPLATE_CONFIG: &str = include_str!("../../config.template");

#[derive(Debug)]
pub struct Config {
    pub prefixes: Vec<char>,
    pub owners: Vec<String>,
    pub mode: Mode,

    /// Raw parsed key-value configuration.
    pub parsed: ParsedConfig,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Public,
    Group,
    Owner,
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

impl Default for Mode {
    fn default() -> Self {
        Self::Public
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

impl Config {
    pub fn load(session: &Session) -> anyhow::Result<Self> {
        let path = ensure_config_file(&session.path)?;
        let content = std::fs::read_to_string(path)?;

        Ok(Self::parse(&content))
    }

    pub fn parse(input: &str) -> Self {
        let parsed = ParsedConfig::parse(input);

        Self {
            prefixes: parsed.get_chars("prefixes"),
            owners: parsed.get_list("owners"),
            mode: parsed
                .get("mode")
                .and_then(|value| value.parse().ok())
                .unwrap_or_default(),
            parsed,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefixes: vec!['.'],
            owners: Vec::new(),
            mode: Mode::default(),
            parsed: ParsedConfig::default(),
        }
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
