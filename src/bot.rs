use std::sync::Arc;

use tokio::fs;
use whatsapp_rust::{TokioRuntime, bot, transport::TokioWebSocketTransportFactory};

use crate::{client::IsahcClient, handler::event_handler, store::RedbStore};

pub async fn run() {
    let backend = RedbStore::new("store.redb").expect("failed to create store");
    let http_client = isahc::HttpClient::builder()
        .build()
        .expect("failed to build isahc http client");

    let config_text = fs::read_to_string("config")
        .await
        .expect("failed to read config");
    let config = Arc::new(viola_core::config::Config::parse(&config_text));

    if let Ok(bot) = bot::Bot::builder()
        .with_http_client(IsahcClient::new())
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_runtime(TokioRuntime)
        .with_backend(backend)
        .skip_history_sync()
        .on_event(move |event, wa_client| {
            let http_client = http_client.clone();
            let config = Arc::clone(&config);

            event_handler(event, wa_client, http_client, config)
        })
        .build()
        .await
    {
        bot.run().await;
    }
}
