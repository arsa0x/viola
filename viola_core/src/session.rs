use std::path::PathBuf;

use whatsapp_rust::anyhow;

use crate::config;
use crate::paths::AppDirs;

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub path: PathBuf,
}

pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    pub fn new(dirs: &AppDirs) -> Self {
        Self {
            root: dirs.sessions.clone(),
        }
    }

    pub fn list(&self) -> anyhow::Result<Vec<String>> {
        let mut sessions = Vec::new();

        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;

            if !entry.file_type()?.is_dir() {
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };

            sessions.push(name);
        }

        sessions.sort();

        Ok(sessions)
    }

    pub fn exists(&self, name: &str) -> anyhow::Result<bool> {
        Ok(self.path(name)?.is_dir())
    }

    pub fn create(&self, name: &str) -> anyhow::Result<Session> {
        validate_name(name)?;

        let path = self.path(name)?;

        if path.exists() {
            return Err(anyhow::anyhow!("session '{name}' already exists"));
        }

        std::fs::create_dir_all(&path)?;
        config::ensure_config_file(&path)?;

        Ok(Session {
            name: name.to_owned(),
            path,
        })
    }

    pub fn get(&self, name: &str) -> anyhow::Result<Session> {
        validate_name(name)?;

        let path = self.path(name)?;

        if !path.is_dir() {
            return Err(anyhow::anyhow!("session '{name}' not found"));
        }

        Ok(Session {
            name: name.to_owned(),
            path,
        })
    }

    pub fn delete(&self, name: &str) -> anyhow::Result<()> {
        let session = self.get(name)?;

        std::fs::remove_dir_all(session.path)?;

        Ok(())
    }

    fn path(&self, name: &str) -> anyhow::Result<PathBuf> {
        validate_name(name)?;
        Ok(self.root.join(name))
    }
}

fn validate_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("session name cannot be empty"));
    }

    if matches!(name, "." | "..") {
        return Err(anyhow::anyhow!("invalid session name: '{name}'"));
    }

    if name.contains(['/', '\\']) {
        return Err(anyhow::anyhow!(
            "session name cannot contain path separators"
        ));
    }

    Ok(())
}
