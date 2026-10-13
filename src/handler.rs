use std::sync::Arc;

use qrcode::render::unicode;
use whatsapp_rust::{Client, types::events::Event};

use crate::{COMMAND_MAP, incoming, parser};

/// Return a phone-number identity only for PN JIDs.
///
/// Owner IDs in the config are phone numbers, not LIDs. In particular, never
/// compare a numeric LID with the configured phone-number allowlist.
fn phone_number_identity(jid: &str, user: &str) -> Option<String> {
    if !jid.ends_with("@s.whatsapp.net") || user.trim().is_empty() {
        return None;
    }

    Some(user.to_owned())
}

/// Resolve the best available phone-number identity for owner checks.
///
/// For LID-addressed messages WhatsApp can provide the phone-number JID in
/// `sender_alt`. If neither JID is a phone-number JID, identity is unknown.
fn sender_id(info: &whatsapp_rust::types::message::MessageInfo) -> Option<String> {
    let source = &info.source;

    source
        .sender_alt
        .as_ref()
        .and_then(|jid| phone_number_identity(&jid.to_string(), &jid.user.to_string()))
        .or_else(|| {
            phone_number_identity(&source.sender.to_string(), &source.sender.user.to_string())
        })
}

pub async fn event_handler(
    session: String,
    event: Arc<Event>,
    wa_client: Arc<Client>,
    http_client: reqwest::Client,
    http_client_no_redirect: reqwest::Client,
    config: Arc<viola_core::Config>,
) {
    match &*event {
        Event::PairingQrCode(qr) => match qrcode::QrCode::new(&qr.code) {
            Ok(qrcode) => {
                let qr_str = qrcode
                    .render::<unicode::Dense1x2>()
                    .quiet_zone(false)
                    .build();
                println!("[{session}] scan for pairing:\n{qr_str}");
            }

            Err(e) => log::error!("[{session}] failed to generate qr: {e}"),
        },

        Event::Messages(batch) => {
            for inb in batch {
                // if let Err(e) = debug::log_message(&inb.info.source.chat.user, &inb.message) {
                //    log::error!("{e}");
                // }
                let Some(text) = incoming::get_text_content(&inb.message) else {
                    continue;
                };

                let Some(prefix) = text.chars().next() else {
                    continue;
                };

                if !config.prefixes.contains(&prefix) {
                    continue;
                }

                let cmd_text = &text[prefix.len_utf8()..];
                let cmd_args = parser::parse(cmd_text);

                if let Some(first) = cmd_args.first()
                    && let Some(&cmd) = COMMAND_MAP.get(first.as_str())
                {
                    let sender_id = sender_id(&inb.info);

                    // Enforce both global mode and command-specific restrictions
                    // before spawning the command. Unknown identity fails closed.
                    if !viola_core::authorization::can_execute_command(
                        config.mode,
                        &config.owners,
                        sender_id.as_deref(),
                        inb.info.source.is_group,
                        cmd.owner_only,
                        cmd.group_only,
                    ) {
                        log::debug!(
                            "denied command '{}' by authorization policy",
                            cmd.name
                        );
                        continue;
                    }

                    let ctx = viola_core::Context {
                        args: cmd_args,
                        http_client: http_client.clone(),
                        http_client_no_redirect: http_client_no_redirect.clone(),
                        wa_client: wa_client.clone(),
                        info: inb.info.clone(),
                        message: inb.message.clone(),
                        config: config.clone(),
                    };

                    tokio::spawn(async move {
                        if let Err(e) = (cmd.execute)(ctx).await {
                            log::error!(
                                "failed to execute command: {} with error error: {:?}",
                                cmd.name,
                                e
                            );
                        };
                    });
                }
            }
        }

        Event::Connected(_) => log::info!("[{session}] Bot connected!"),

        Event::Disconnected(_) => log::info!("[{session}] Bot was disconnected!"),

        Event::LoggedOut(_) => log::info!("[{session}] Bot was logged out!"),

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::sender_id;
    use whatsapp_rust::types::message::MessageInfo;

    fn info_with_sender(sender: &str, sender_alt: Option<&str>) -> MessageInfo {
        let mut info = MessageInfo::default();
        info.source.sender = sender.parse().expect("valid sender JID");
        info.source.sender_alt = sender_alt.map(|jid| jid.parse().expect("valid alternate JID"));
        info
    }

    #[test]
    fn sender_id_uses_phone_number_alternate_for_lid_sender() {
        let info = info_with_sender(
            "203040904720543@lid",
            Some("628123456789@s.whatsapp.net"),
        );

        assert_eq!(sender_id(&info).as_deref(), Some("628123456789"));
    }

    #[test]
    fn sender_id_rejects_lid_when_phone_number_is_unavailable() {
        let info = info_with_sender("203040904720543@lid", None);

        assert_eq!(sender_id(&info), None);
    }

    #[test]
    fn sender_id_accepts_phone_number_sender_without_alternate() {
        let info = info_with_sender("628123456789@s.whatsapp.net", None);

        assert_eq!(sender_id(&info).as_deref(), Some("628123456789"));
    }
}
