use crate::{client::ReqwestClient, handler::event_handler, store::RedbStore};
use std::sync::Arc;
use tokio::task::JoinSet;
use viola_core::{config, session};
use whatsapp_rust::{TokioRuntime, bot, transport::TokioWebSocketTransportFactory};

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");
}

pub async fn run_sessions(names: Vec<String>) {
    if names.len() == 1 {
        run_one(names.into_iter().next().unwrap()).await;
        return;
    }
    let mut set = JoinSet::new();
    for name in names {
        set.spawn(run_one(name));
    }

    tokio::select! {
      _ = async {
        while set.join_next().await.is_some() {}
      } => {},

      _ = shutdown_signal() => {
        log::info!("Shutdown signal received. Stopping all sessions...");
        set.abort_all();
        while set.join_next().await.is_some() {}
        }
    }
}

async fn run_one(name: String) {
    let dir = match session::ensure_session_dir(&name) {
        Ok(d) => d,
        Err(err) => {
            log::error!("[{name}] failed to prepare session directory: {err}");
            return;
        }
    };

    let backend = match RedbStore::new(&dir.join("store.redb").to_string_lossy()) {
        Ok(b) => b,
        Err(err) => {
            log::error!("[{name}] failed to open store: {err}");
            return;
        }
    };

    let config = match config::load_for_session(&name) {
        Ok(c) => Arc::new(c),
        Err(err) => {
            log::error!("[{name}] failed to load configuration: {err}");
            return;
        }
    };

    let http_client = reqwest::Client::builder()
        .build()
        .expect("failed to build reqwest client");
    let http_client_no_redirect = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build no-redirect reqwest client");

    let session_name = name.clone();
    let bot = bot::Bot::builder()
        .with_http_client(ReqwestClient::new())
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_runtime(TokioRuntime)
        .with_backend(backend)
        .skip_history_sync()
        .on_event(move |event, wa_client| {
            let http_client = http_client.clone();
            let http_client_no_redirect = http_client_no_redirect.clone();
            let config = Arc::clone(&config);
            let session_name = session_name.clone();
            event_handler(
                session_name,
                event,
                wa_client,
                http_client,
                http_client_no_redirect,
                config,
            )
        })
        .build()
        .await;

    match bot {
        Ok(bot) => {
            log::info!("[{name}] bot started");
            bot.run().await;
        }
        Err(err) => log::error!("[{name}] failed to initialize bot: {err}"),
    }
}
