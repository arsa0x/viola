use wacore::proto_helpers::MessageExt;
use waproto::whatsapp::Message;

fn split_arguments(input: &str) -> Vec<String> {
    // let mut tokens = Vec::new();
    // let mut current = String::new();
    let mut tokens = Vec::with_capacity(4);
    let mut current = String::with_capacity(32);

    let mut in_quotes = false;

    for c in input.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\n' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub fn text_content(msg: &Message) -> Option<&str> {
    if let Some(text) = msg.text_content() {
        return Some(text);
    }

    if let Some(image) = &msg.image_message {
        if let Some(caption) = &image.caption {
            return Some(&*caption);
        }
    }

    if let Some(video) = &msg.video_message {
        if let Some(caption) = &video.caption {
            return Some(caption);
        }
    }

    if let Some(document) = &msg.document_message {
        if let Some(caption) = &document.caption {
            return Some(caption);
        }
    }

    None
}

pub fn parse_command(prefix: &str, msg: &Message) -> Option<(String, Vec<String>)> {
    let text = text_content(msg)?;

    if !text.starts_with(prefix) {
        return None;
    }
    let without_prefix = text.trim_start_matches(prefix);
    let mut parts: Vec<String> = split_arguments(without_prefix);

    let cmd = parts.remove(0).to_lowercase();

    Some((cmd, parts))
}
