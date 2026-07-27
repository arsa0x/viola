use std::path::PathBuf;

use directories::ProjectDirs;
use whatsapp_rust::anyhow;

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("", "", "viola").expect("failed to find config dir")
}

pub fn sessions_root() -> std::path::PathBuf {
    project_dirs().config_dir().join("sessions")
}

pub fn session_path(name: &str) -> PathBuf {
    sessions_root().join(name)
}

pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let root = sessions_root();

    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.into());
            }
        }
    }

    names.sort();
    Ok(names)
}

pub fn ensure_session_dir(name: &str) -> anyhow::Result<PathBuf> {
    let dir = session_path(name);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
