use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use whatsapp_rust::{
    anyhow,
    chrono::Local,
    serde_json::{self, json},
};

#[allow(unused)]
pub fn log_message(jid: &str, msg: &impl serde::Serialize) -> anyhow::Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let dir = format!("logs/{}", jid);
    fs::create_dir_all(&dir)?;
    let file_path = format!("{}/{}.jsonl", dir, date);
    let entry = json!({
        "timestamp": Local::now().to_rfc3339(),
        "message": msg
    });
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    writeln!(file, "{}", serde_json::to_string(&entry)?)?;

    Ok(())
}
