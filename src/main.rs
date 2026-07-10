use qrcode::render::unicode;
use std::{io::Write, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use viola::{client::IsahcClient, incoming, store::RedbStore};
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
    types::events::{Event, EventKind},
};

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let dir = init_dir()?;

        let load_conf = load_config(&dir.join("config.toml").to_string_lossy())?;

        let config = Arc::new(RwLock::new(load_conf));

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

        log::info!("Redb backend initialized");
        log::info!("Starting bot...");

        let state_for_bot = state.clone();

        let bot = Bot::builder()
            .with_backend(backend)
            .with_transport_factory(TokioWebSocketTransportFactory::new())
            .with_http_client(IsahcClient::new(isahc_client))
            .with_runtime(TokioRuntime)
            .on_event_for(
                &[
                    EventKind::PairingCode,
                    EventKind::Connected,
                    EventKind::Messages,
                    EventKind::Disconnected,
                    EventKind::LoggedOut,
                ],
                move |event, client| {
                    let state = state_for_bot.clone();
                    async move {
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

                            Event::Connected(_) => log::info!("Bot connected!"),

                            Event::Messages(batch) => {
                                for inb in batch {
                                    let msg = inb.message.clone();
                                    let info = inb.info.clone();

                                    let text_content = match incoming::get_text_content(&msg) {
                                        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
                                        _ => continue,
                                    };

                                    if let Ok(permit) = state.semaphore.clone().try_acquire_owned()
                                    {
                                        let state_spawn = state.clone();
                                        let client_spawn = client.clone();
                                        let start_time = Instant::now();

                                        tokio::spawn(async move {
                                            let _permit = permit;

                                            let prefix = {
                                                let config = state_spawn.config.read().await;
                                                config.bot.prefix.clone()
                                            };

                                            let internal_msg_ctx = MessageContext::from_parts(
                                                &msg,
                                                &info,
                                                client_spawn,
                                            );
                                            let ctx = Context {
                                                msg_ctx: internal_msg_ctx,
                                                args: Vec::new(),
                                                state: state_spawn.clone(),
                                                created_at: start_time,
                                            };

                                            if let Some((command, args)) =
                                                viola::parser::parse(&prefix, &text_content)
                                            {
                                                let mut ctx_with_args = ctx;
                                                ctx_with_args.args = args;

                                                if let Err(e) = state_spawn
                                                    .router
                                                    .execute(&command, ctx_with_args)
                                                    .await
                                                {
                                                    log::error!(
                                                        "command [{}] failed: {}",
                                                        command,
                                                        e
                                                    );
                                                }
                                            }
                                        });
                                    } else {
                                        log::warn!("Server sangat sibuk, menolak pesan.");
                                    }
                                }
                            }
                            Event::Disconnected(_) => log::info!("Bot was disconnected!"),

                            Event::LoggedOut(_) => log::info!("Bot was logged out!"),
                            _ => {}
                        }
                    }
                },
            )
            .build()
            .await?;

        bot.run().await;

        log::info!("bot is running. press ctrl+c to stop.");

        Ok(())
    })
}
