mod client;
mod store;
mod utils;

use crate::{client::IsahcClient, store::redb_store::RedbStore};
use qrcode::render::unicode;
use std::{io::Write, sync::Arc, time::Instant};
use viola_core::{
    config::{init_dir, load_config},
    context::Context,
    router::Router,
    state::AppState,
};
use viola_plugin as _;
use whatsapp_rust::{
    TokioRuntime,
    bot::{Bot, MessageContext},
    transport::TokioWebSocketTransportFactory,
    types::events::Event,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let store_path = dir.join("store.redb");
    let backend = RedbStore::new(&store_path.to_string_lossy())?;

    let router = Arc::new(Router::new());

    let isahc_client = isahc::HttpClient::builder().build()?;

    let state = Arc::new(AppState::new(
        Arc::clone(&config),
        Arc::clone(&router),
        Arc::new(dir),
        isahc_client.clone(),
    ));

    log::info!("SQLite backend initialized");

    log::info!("Starting bot...");

    let bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(IsahcClient::new(isahc_client))
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
                        let start_time = Instant::now();
                        let prefix = &state.config.bot.prefix;

                        if let Some((command, args)) = utils::parse_command(prefix, &msg) {
                            let state_handler = state.clone();

                            if let Ok(permit) = state.semaphore.clone().try_acquire_owned() {
                                let ctx = Context {
                                    msg_ctx: MessageContext::from_parts(msg, info, client.clone()),
                                    args,
                                    state,
                                    created_at: start_time,
                                };
                                tokio::spawn(async move {
                                    let _permit = permit;

                                    if let Err(e) =
                                        state_handler.router.execute(&command, ctx).await
                                    {
                                        log::error!("command failed: {}", e);
                                    }
                                });
                            } else {
                                log::error!("server is busy");
                            };
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

    bot.run().await;

    log::info!("bot is running. press ctrl+c to stop.");

    Ok(())
}
