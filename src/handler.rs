use std::sync::Arc;

use qrcode::render::unicode;
use whatsapp_rust::{Client, types::events::Event};

use crate::{COMMAND_MAP, incoming, parser};

pub async fn event_handler(
    event: Arc<Event>,
    wa_client: Arc<Client>,
    http_client: isahc::HttpClient,
    config: Arc<viola_core::Config>,
) {
    match &*event {
        Event::PairingQrCode(qr) => match qrcode::QrCode::new(&qr.code) {
            Ok(qrcode) => {
                let qr_str = qrcode
                    .render::<unicode::Dense1x2>()
                    .quiet_zone(false)
                    .build();
                println!("{}", qr_str);
            }

            Err(e) => log::error!("failed to generate qr: {}", e),
        },

        Event::Messages(batch) => {
            for inb in batch {
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

                match config.mode {
                    viola_core::Mode::Group => {
                        if !inb.info.source.is_group {
                            continue;
                        }
                    }
                    viola_core::Mode::Owner => {
                        if let Some(sender) = &inb.info.source.sender_alt {
                            if !config.owners.contains(&sender.to_string()) {
                                continue;
                            }
                        }
                    }
                    viola_core::Mode::Public => {}
                }

                let cmd_args = parser::parse(cmd_text);

                if let Some(first) = cmd_args.first()
                    && let Some(&cmd) = COMMAND_MAP.get(first.as_str())
                {
                    let ctx = viola_core::Context {
                        args: cmd_args,
                        http_client: http_client.clone(),
                        wa_client: wa_client.clone(),
                        info: inb.info.clone(),
                        message: inb.message.clone(),
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

        Event::Connected(_) => log::info!("Bot connected!"),

        Event::Disconnected(_) => log::info!("Bot was disconnected!"),

        Event::LoggedOut(_) => log::info!("Bot was logged out!"),

        _ => {}
    }
}
