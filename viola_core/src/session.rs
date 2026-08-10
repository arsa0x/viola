use std::path::PathBuf;

use directories::ProjectDirs;
use whatsapp_rust::anyhow;

fn sessions_root() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("", "", "viola")
        .ok_or_else(|| anyhow::anyhow!("failed to determine application directory"))?;

    let root = project_dirs.config_dir().join("sessions");

    std::fs::create_dir_all(&root)?;

    Ok(root)
}

pub fn session_path(name: &str) -> anyhow::Result<PathBuf> {
    validate_session_name(name)?;

    Ok(sessions_root()?.join(name))
}

pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let root = sessions_root()?;

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(root)? {
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

pub fn ensure_session_dir(name: &str) -> anyhow::Result<PathBuf> {
    let dir = session_path(name)?;

    std::fs::create_dir_all(&dir)?;

    Ok(dir)
}

pub fn remove_session(name: &str) -> anyhow::Result<()> {
    let dir = session_path(name)?;

    if !dir.exists() {
        return Err(anyhow::anyhow!("session '{name}' not found"));
    }

    std::fs::remove_dir_all(dir)?;

    Ok(())
}

fn validate_session_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("session name cannot be empty"));
    }

    if name == "." || name == ".." {
        return Err(anyhow::anyhow!("invalid session name: '{name}'"));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(anyhow::anyhow!(
            "session name cannot contain path separators"
        ));
    }

    Ok(())
}
