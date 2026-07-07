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
use whatsapp_rust::{TokioRuntime, bot::Bot, transport::TokioWebSocketTransportFactory};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        .on_qr_code(|code, _timeout| async move {
            match qrcode::QrCode::new(code.as_bytes()) {
                Ok(qr) => {
                    let qr_str = qr.render::<unicode::Dense1x2>().quiet_zone(false).build();
                    println!("{}", qr_str);
                }
                Err(e) => {
                    log::error!("failed to generate qr: {}", e);
                }
            }
        })
        .on_message(move |msg_ctx| {
            let state = state_for_bot.clone();

            async move {
                let start = Instant::now();

                let text_content = match incoming::get_text_content(&msg_ctx.message) {
                    Some(t) if !t.trim().is_empty() => t.trim().to_string(),
                    _ => return,
                };

                if let Ok(permit) = state.semaphore.clone().try_acquire_owned() {
                    tokio::spawn(async move {
                        let _permit = permit;

                        let prefix = {
                            let config = state.config.read().await;
                            config.bot.prefix.clone()
                        };

                        let ctx = Context {
                            msg_ctx,
                            args: Vec::new(),
                            state: state.clone(),
                            created_at: start,
                        };

                        if let Some((command, args)) = viola::parser::parse(&prefix, &text_content)
                        {
                            let mut ctx_with_args = ctx;
                            ctx_with_args.args = args;

                            if let Err(e) = state.router.execute(&command, ctx_with_args).await {
                                log::error!("command failed: {}", e);
                            }
                        } else {
                            return;
                        }
                    });
                } else {
                    log::warn!("server is busy");
                };
            }
        })
        .build()
        .await?;

    bot.run().await;

    log::info!("bot is running. press ctrl+c to stop.");

    Ok(())
}
