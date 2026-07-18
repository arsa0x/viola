use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

const TEMPLATE_CONFIG: &str = include_str!("../../config.template");

pub const CONFIG_FILE: &str = "config";
pub const DOWNLOAD_DIR: &str = "download";
pub const CACHE_DIR: &str = "cache";

pub fn binary_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "viola".to_string())
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
        let content = fs::read_to_string(CONFIG_FILE)?;
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

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    if let Err(err) = validate_project_dir() {
        log::error!("{err}");
        std::process::exit(1);
    }

    Config::load().unwrap_or_else(|err| {
        log::error!("failed to read config file: {err}");
        std::process::exit(1);
    })
});

pub fn init_project() -> std::io::Result<PathBuf> {
    let project_name = binary_name();
    let root = Path::new(&project_name);

    if root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "'./{project_name}' already exists, remove or rename it before running init again"
            ),
        ));
    }

    fs::create_dir(root)?;
    fs::create_dir(root.join(DOWNLOAD_DIR))?;
    fs::create_dir(root.join(CACHE_DIR))?;
    fs::write(root.join(CONFIG_FILE), TEMPLATE_CONFIG)?;

    Ok(root.to_path_buf())
}

pub fn validate_project_dir() -> std::io::Result<()> {
    let name = binary_name();
    let mut missing = Vec::new();

    if !Path::new(CONFIG_FILE).is_file() {
        missing.push(CONFIG_FILE);
    }
    if !Path::new(DOWNLOAD_DIR).is_dir() {
        missing.push(DOWNLOAD_DIR);
    }
    if !Path::new(CACHE_DIR).is_dir() {
        missing.push(CACHE_DIR);
    }

    if missing.is_empty() {
        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "this doesn't look like a {name} project directory (missing: {}).\n\
             run `{name} init` first, then `cd` into the generated folder before starting the bot.",
            missing.join(", ")
        ),
    ))
}
