use std::sync::Arc;

use tokio::task::JoinSet;
use viola_core::{config, session};
use whatsapp_rust::{TokioRuntime, bot, transport::TokioWebSocketTransportFactory};

use crate::{client::ReqwestClient, handler::event_handler, store::RedbStore};

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        log::error!("failed to listen for Ctrl+C: {err}");
    }
}

pub async fn run_sessions(names: Vec<String>) {
    if names.is_empty() {
        return;
    }

    if names.len() == 1 {
        if let Some(name) = names.into_iter().next() {
            run_one(name).await;
        }

        return;
    }

    let mut tasks = JoinSet::new();

    for name in names {
        tasks.spawn(run_one(name));
    }

    tokio::select! {
        _ = wait_for_tasks(&mut tasks) => {}

        _ = shutdown_signal() => {
            log::info!(
                "Shutdown signal received. \
                 Stopping all sessions..."
            );

            tasks.abort_all();

            while let Some(result) = tasks.join_next().await {
                if let Err(err) = result {
                    if !err.is_cancelled() {
                        log::error!(
                            "session task failed: {err}"
                        );
                    }
                }
            }
        }
    }
}

async fn wait_for_tasks(tasks: &mut JoinSet<()>) {
    while let Some(result) = tasks.join_next().await {
        if let Err(err) = result {
            log::error!("session task failed: {err}");
        }
    }
}

async fn run_one(name: String) {
    let dir = match session::ensure_session_dir(&name) {
        Ok(dir) => dir,

        Err(err) => {
            log::error!("[{name}] failed to prepare session directory: {err}");

            return;
        }
    };

    let store_path = dir.join("store.redb");

    let backend = match RedbStore::new(&store_path.to_string_lossy()) {
        Ok(backend) => backend,

        Err(err) => {
            log::error!("[{name}] failed to open store: {err}");

            return;
        }
    };

    let config = match config::load_for_session(&name) {
        Ok(config) => Arc::new(config),

        Err(err) => {
            log::error!("[{name}] failed to load configuration: {err}");

            return;
        }
    };

    let http_client = match reqwest::Client::builder().build() {
        Ok(client) => client,

        Err(err) => {
            log::error!("[{name}] failed to build HTTP client: {err}");

            return;
        }
    };

    let http_client_no_redirect = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,

        Err(err) => {
            log::error!("[{name}] failed to build no-redirect HTTP client: {err}");

            return;
        }
    };

    let session_name = name.clone();

    let bot = bot::Bot::builder()
        .with_http_client(ReqwestClient::new(http_client.clone()))
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_runtime(TokioRuntime)
        .with_backend(backend)
        .skip_history_sync()
        .on_event(move |event, wa_client| {
            event_handler(
                session_name.clone(),
                event,
                wa_client,
                http_client.clone(),
                http_client_no_redirect.clone(),
                Arc::clone(&config),
            )
        })
        .build()
        .await;

    match bot {
        Ok(bot) => {
            log::info!("[{name}] bot started");

            bot.run().await;

            log::info!("[{name}] bot stopped");
        }

        Err(err) => {
            log::error!("[{name}] failed to initialize bot: {err}");
        }
    }
}
