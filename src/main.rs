use qrcode::render::unicode;
use std::{io::Write, sync::Arc};
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use viola_core::{
    config::{init_dir, load_config},
    {context::Context, router::Router, state::AppState},
};
use viola_plugin as _;
use whatsapp_rust::{
    TokioRuntime,
    bot::Bot,
    store::SqliteStore,
    transport::{TokioWebSocketTransportFactory, UreqHttpClient},
    types::events::Event,
};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Arc::new(Router::new());

    let dir = init_dir()?;
    let config = Arc::new(load_config(&dir.join("config.toml").to_string_lossy())?);

    let http_client = reqwest::Client::builder().cookie_store(true).build()?;

    let http_no_redirect = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let state = Arc::new(AppState::new(
        Arc::clone(&config),
        Arc::clone(&router),
        Arc::new(http_client),
        Arc::new(http_no_redirect),
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

                        if let Some(ctx) =
                            Context::new(msg, info, client, state.clone()).parse_command(prefix)
                        {
                            let state_handler = state.clone();

                            tokio::spawn(async move {
                                let _permit = state_handler
                                    .semaphore
                                    .acquire()
                                    .await
                                    .expect("semaphore closed");

                                let command = ctx.command.clone();

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
