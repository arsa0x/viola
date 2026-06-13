use compact_str::CompactString;
use waproto::whatsapp::Message;

fn split_arguments(input: &str) -> Vec<CompactString> {
    let mut result = Vec::with_capacity(4);

    let mut in_quotes = false;
    let mut start = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => {
                if in_quotes {
                    if let Some(s) = start {
                        result.push(CompactString::from(&input[s..idx]));
                    }
                    start = None;
                } else {
                    start = Some(idx + 1);
                }

                in_quotes = !in_quotes;
            }
            ' ' | '\n' if !in_quotes => {
                if let Some(s) = start {
                    result.push(CompactString::from(&input[s..idx]));
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
        result.push(CompactString::from(&input[s..]));
    }
    result
}

pub fn parse_command(prefix: &str, msg: &Message) -> Option<(CompactString, Vec<CompactString>)> {
    let text = viola_core::utils::get_text_content(msg)?;

    if !text.starts_with(prefix) {
        return None;
    }

    let without_prefix = text.trim_start_matches(prefix);
    let parts = self::split_arguments(without_prefix);
    let (cmd, args) = parts.split_first()?;

    Some((cmd.clone(), args.to_vec()))
}
