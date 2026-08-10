use std::sync::Arc;

use qrcode::render::unicode;
use viola_core::plugin;
use whatsapp_rust::{Client, types::events::Event};

use crate::{COMMAND_MAP, incoming, parser};

pub async fn event_handler(
    session: String,
    event: Arc<Event>,
    wa_client: Arc<Client>,
    http_client: reqwest::Client,
    http_client_no_redirect: reqwest::Client,
    config: Arc<viola_core::Config>,
    plugins: Arc<plugin::PluginRegistry>,
) {
    match &*event {
        Event::PairingQrCode(qr) => match qrcode::QrCode::new(&qr.code) {
            Ok(qr_code) => {
                let qr = qr_code
                    .render::<unicode::Dense1x2>()
                    .quiet_zone(false)
                    .build();

                println!("[{session}] scan for pairing:\n{qr}");
            }

            Err(err) => {
                log::error!("[{session}] failed to generate QR code: {err}");
            }
        },

        Event::Messages(batch) => {
            for incoming in batch {
                let Some(text) = incoming::get_text_content(&incoming.message) else {
                    continue;
                };

                let Some(prefix) = text.chars().next() else {
                    continue;
                };

                if !config.prefixes.contains(&prefix) {
                    continue;
                }

                match config.mode {
                    viola_core::Mode::Group => {
                        if !incoming.info.source.is_group {
                            continue;
                        }
                    }

                    viola_core::Mode::Owner => {
                        let Some(sender) = &incoming.info.source.sender_alt else {
                            continue;
                        };

                        if !config.owners.contains(&sender.user.to_string()) {
                            continue;
                        }
                    }

                    viola_core::Mode::Public => {}
                }

                let command_text = &text[prefix.len_utf8()..];
                let args = parser::parse(command_text);

                let Some(trigger) = args.first() else {
                    continue;
                };

                if let Some(&command) = COMMAND_MAP.get(trigger.as_str()) {
                    let ctx = viola_core::Context {
                        args,
                        http_client: http_client.clone(),
                        http_client_no_redirect: http_client_no_redirect.clone(),
                        wa_client: wa_client.clone(),
                        info: incoming.info.clone(),
                        message: incoming.message.clone(),
                        config: config.clone(),
                    };

                    tokio::spawn(async move {
                        if let Err(err) = (command.execute)(ctx).await {
                            log::error!("failed to execute command '{}': {err:?}", command.name);
                        }
                    });

                    continue;
                }

                let Some(chunk) = plugins.find(trigger.as_str()) else {
                    continue;
                };

                let trigger = trigger.clone();

                let ctx = viola_core::Context {
                    args,
                    http_client: http_client.clone(),
                    http_client_no_redirect: http_client_no_redirect.clone(),
                    wa_client: wa_client.clone(),
                    info: incoming.info.clone(),
                    message: incoming.message.clone(),
                    config: config.clone(),
                };

                tokio::spawn(async move {
                    if let Err(err) = plugin::dispatch(ctx, &chunk).await {
                        log::error!("failed to execute plugin trigger '{}': {err:?}", trigger);
                    }
                });
            }
        }

        Event::Connected(_) => {
            log::info!("[{session}] Bot connected!");
        }

        Event::Disconnected(_) => {
            log::info!("[{session}] Bot was disconnected!");
        }

        Event::LoggedOut(_) => {
            log::info!("[{session}] Bot was logged out!");
        }

        _ => {}
    }
}
