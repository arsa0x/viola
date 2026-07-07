pub fn parse(prefix: &str, message: &str) -> Option<(String, Vec<String>)> {
    if !message.starts_with(prefix) {
        return None;
    }

    let without_prefix = message.trim_start_matches(prefix);
    let parts = split_message(without_prefix);

    let (cmd, args) = parts.split_first()?;

    Some((cmd.to_string(), args.to_vec()))
}

fn split_message(message: &str) -> Vec<String> {
    let mut result = Vec::new();

    let mut in_quotes = false;
    let mut start = None;

    for (idx, ch) in message.char_indices() {
        match ch {
            '"' => {
                if in_quotes {
                    if let Some(s) = start {
                        result.push(String::from(&message[s..idx]));
                    }
                    start = None;
                } else {
                    start = Some(idx + 1);
                }

                in_quotes = !in_quotes;
            }
            ' ' | '\n' if !in_quotes => {
                if let Some(s) = start {
                    result.push(String::from(&message[s..idx]));
                    start = None;
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(idx);
                }
            }
        }
    }
    if let Some(s) = start {
        result.push(String::from(&message[s..]));
    }
    result
}
