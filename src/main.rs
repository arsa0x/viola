use qrcode::render::unicode;
use std::{io::Write, sync::Arc};
use waproto::whatsapp;
use whatsapp_rust::{
    TokioRuntime,
    bot::{Bot, MessageContext},
    proto_helpers::MessageExt,
    types::events::Event,
};
use whatsapp_rust_sqlite_storage::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

const PING: &str = ".ping";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{:<5}] [{}] - {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();

    let backend = Arc::new(SqliteStore::new("store/whatsapp.db").await?);
    println!("SQLite backend initialized");

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(|event, client| async move {
            match &*event {
                Event::PairingQrCode { code, .. } => {
                    let qr = qrcode::QrCode::new(code.as_bytes()).unwrap();
                    let qr_str = qr.render::<unicode::Dense1x2>().quiet_zone(false).build();
                    println!("{}", qr_str);
                }
                Event::Message(msg, info) => {
                    let ctx = MessageContext::from_parts(msg, info, client);

                    if ctx.message.text_content() == Some(PING) {
                        let ctx_info = ctx.build_quote_context();
                        let reply = whatsapp::Message {
                            extended_text_message: Some(Box::new(
                                whatsapp::message::ExtendedTextMessage {
                                    text: Some("pong".to_string()),
                                    context_info: Some(Box::new(ctx_info)),
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        };
                        if let Err(e) = ctx.send_message(reply).await {
                            println!("failed to send message: {}", e);
                        }
                    }
                }
                Event::Connected(_) => println!("Bot connected!"),
                Event::LoggedOut(_) => println!("Bot was logged out!"),
                _ => {}
            }
        })
        .build()
        .await?;
    println!("Starting bot...");
    bot.run().await?.await?;
    Ok(())
}
