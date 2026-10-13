use std::sync::Arc;

use qrcode::render::unicode;
use whatsapp_rust::{Client, types::events::Event};

use crate::{COMMAND_MAP, incoming, parser};

/// Resolve the best available sender identifier for owner checks.
///
/// WhatsApp may provide a phone-number JID as `sender_alt` when the primary
/// sender JID is a LID. The configured `owners` list contains phone numbers
/// without a leading plus sign. If the alternate identity is absent, fall
/// back to the primary sender identifier; an empty identifier is unknown.
fn sender_id(info: &whatsapp_rust::types::message::MessageInfo) -> Option<String> {
    let source = &info.source;

    source
        .sender_alt
        .as_ref()
        .map(|sender| sender.user.to_string())
        .or_else(|| {
            let sender = source.sender.user.to_string();
            (!sender.trim().is_empty()).then_some(sender)
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
