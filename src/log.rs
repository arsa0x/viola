use chrono::Local;
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
};
use whatsapp_rust::waproto::whatsapp;

pub fn log_message(jid: &str, msg_type: &str, msg: &impl serde::Serialize) -> anyhow::Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let dir = format!("logs/{}", jid);
    fs::create_dir_all(&dir)?;
    let file_path = format!("{}/{}.jsonl", dir, date);
    let entry = json!({
        "type": msg_type,
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

pub fn detect_message_type(msg: &whatsapp::Message) -> &'static str {
    if msg.conversation.is_some() {
        "conversation"
    } else if msg.extended_text_message.is_some() {
        "extended_text_message"
    } else if msg.image_message.is_some() {
        "image_message"
    } else if msg.video_message.is_some() {
        "video_message"
    } else if msg.document_message.is_some() {
        "document_message"
    } else if msg.list_message.is_some() {
        "list_message"
    } else if msg.buttons_message.is_some() {
        "buttons_message"
    } else if msg.interactive_message.is_some() {
        "interactive_message"
    } else if msg.poll_creation_message.is_some() {
        "poll_creation_message"
    } else {
        "unknown"
    }
}
