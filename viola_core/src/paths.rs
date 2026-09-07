use std::path::PathBuf;

use directories::ProjectDirs;
use whatsapp_rust::anyhow;

#[derive(Debug, Clone)]
pub struct AppDirs {
    pub config: PathBuf,
    pub sessions: PathBuf,
    pub downloads: PathBuf,
    pub plugins: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    pub fn new() -> anyhow::Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "viola")
            .ok_or_else(|| anyhow::anyhow!("failed to determine application directory"))?;

        let config = project_dirs.config_dir().to_path_buf();

        Ok(Self {
            sessions: config.join("sessions"),
            downloads: config.join("downloads"),
            plugins: config.join("plugins"),
            cache: config.join("cache"),
            config,
        })
    }

    pub fn ensure(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.sessions)?;
        std::fs::create_dir_all(&self.downloads)?;
        std::fs::create_dir_all(&self.plugins)?;
        std::fs::create_dir_all(&self.cache)?;

        Ok(())
    }
}
