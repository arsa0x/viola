use qrcode::render::unicode;
use std::{io::Write, sync::Arc};
use viola::{
    framework::{context::Context, router::Router},
    utils::config::{init_dir, load_config},
};
use whatsapp_rust::{TokioRuntime, bot::Bot, types::events::Event};
use whatsapp_rust_sqlite_storage::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Arc::new(Router::new());
    // let semaphore = Arc::new(Semaphore::new(100));

    let dir = init_dir()?;
    let config = Arc::new(load_config(&dir.join("config.toml").to_string_lossy())?);

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Always)
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

    let store_path = dir.join("store").join("whatsapp.db");
    let backend = Arc::new(SqliteStore::new(&store_path.to_string_lossy()).await?);

    log::info!("SQLite backend initialized");

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(move |event, client| {
            let router = Arc::clone(&router);
            let config = Arc::clone(&config);

            async move {
                match &*event {
                    Event::PairingQrCode { code, .. } => {
                        match qrcode::QrCode::new(code.as_bytes()) {
                            Ok(qr) => {
                                let qr_str =
                                    qr.render::<unicode::Dense1x2>().quiet_zone(false).build();
                                println!("{}", qr_str);
                            }
                            Err(e) => {
                                log::error!("failed to generate qr: {}", e);
                            }
                        }
                    }
                    Event::Message(msg, info) => {
                        let ctx = Context::new(msg, info, client, Arc::clone(&config))
                            .parse_command(&config.bot.prefix);

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
                    Event::Connected(_) => log::info!("Bot connected!"),
                    Event::LoggedOut(_) => log::info!("Bot was logged out!"),
                    _ => {}
                }
            }
        })
        .build()
        .await?;
    log::info!("Starting bot...");
    Ok(bot.run().await?.await?)
}
