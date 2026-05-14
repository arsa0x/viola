use qrcode::render::unicode;
use std::{io::Write, sync::Arc};
use viola::framework::{context::Context, router::Router};
use whatsapp_rust::{TokioRuntime, bot::Bot, types::events::Event};
use whatsapp_rust_sqlite_storage::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Arc::new(Router::new());

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
        .on_event(move |event, client| {
            let router = Arc::clone(&router);

            async move {
                match &*event {
                    Event::PairingQrCode { code, .. } => {
                        let qr = qrcode::QrCode::new(code.as_bytes()).unwrap();
                        let qr_str = qr.render::<unicode::Dense1x2>().quiet_zone(false).build();
                        println!("{}", qr_str);
                    }
                    Event::Message(msg, info) => {
                        let ctx = Context::new(msg, info, client).parse_command(".");

                        if !ctx.command.is_empty() {
                            let router = Arc::clone(&router);
                            let cmd = ctx.command.clone();
                            tokio::spawn(async move {
                                if let Err(e) = router.execute(&cmd, ctx).await {
                                    log::error!("command failed: {}", e);
                                }
                            });
                        }
                    }
                    Event::Connected(_) => println!("Bot connected!"),
                    Event::LoggedOut(_) => println!("Bot was logged out!"),
                    _ => {}
                }
            }
        })
        .build()
        .await?;
    println!("Starting bot...");
    bot.run().await?.await?;
    Ok(())
}
