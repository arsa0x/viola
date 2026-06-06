use crate::{config::Config, router::Router};
use reqwest::Client;
use std::{sync::Arc, time::Instant};
use tokio::sync::Semaphore;

pub struct AppState {
    pub config: Arc<Config>,
    pub router: Arc<Router>,
    pub start_time: Instant,
    pub semaphore: Arc<Semaphore>,
    pub http: Client,
    pub http_no_redirect: Client,
}

impl AppState {
    pub fn new(
        config: Arc<Config>,
        router: Arc<Router>,
        http_client: Client,
        http_no_redirect: Client,
    ) -> Self {
        Self {
            config,
            router,
            start_time: Instant::now(),
            semaphore: Arc::new(Semaphore::new(100)),
            http: http_client,
            http_no_redirect: http_no_redirect,
        }
    }
}
