use mimalloc::MiMalloc;
use qrcode::render::unicode;
use std::{io::Write, sync::Arc};
use viola_commands as _;
use viola_core::{
    framework::{
        context::Context,
        router::Router,
        state::{AppState, SharedState},
    },
    utils::config::{init_dir, load_config},
};
use whatsapp_rust::{TokioRuntime, bot::Bot, types::events::Event};
use whatsapp_rust_sqlite_storage::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Arc::new(Router::new());

    let dir = init_dir()?;
    let config = Arc::new(load_config(&dir.join("config.toml").to_string_lossy())?);

    let client = Arc::new(isahc::HttpClient::new()?);

    let state = SharedState::new(AppState::new(
        Arc::clone(&config),
        Arc::clone(&router),
        client,
    ));

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

    log::info!("Starting bot...");

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(move |event, client| {
            let state = Arc::clone(&state);

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
                        let prefix = &state.config.bot.prefix;
                        let ctx =
                            Context::new(msg, info, client, state.clone()).parse_command(prefix);

                        if !ctx.command.is_empty() {
                            let state_handler = state.clone();
                            let command = ctx.command.clone();
                            tokio::spawn(async move {
                                if let Err(e) = state_handler.router.execute(&command, ctx).await {
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

    let mut bot_handle = bot.run().await?;

    log::info!("bot is running. press ctrl+c to stop.");

    // https://github.com/vrypt-cpp/sora-on-rust/blob/main/src/main.rs#L61
    tokio::select! {
        res = &mut bot_handle => {
            match res {
                Ok(_) => log::info!("bot stopped normally."),
                Err(e) => log::error!("bot stopped with error: {:?}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("SIGINT received, performing graceful shutdown...");
            bot.client().disconnect().await;
            let _ = bot_handle.await;
            log::info!("shutdown complete. goodbye!");
        }
    }

    Ok(())
}
