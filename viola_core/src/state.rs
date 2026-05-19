use crate::{config::Config, router::Router};
use std::{sync::Arc, time::Instant};
use tokio::sync::Semaphore;

pub struct AppState {
    pub config: Arc<Config>,
    pub router: Arc<Router>,
    pub start_time: Instant,
    pub semaphore: Arc<Semaphore>,
    pub http: Arc<isahc::HttpClient>,
}

impl AppState {
    pub fn new(config: Arc<Config>, router: Arc<Router>, client: Arc<isahc::HttpClient>) -> Self {
        Self {
            config,
            router,
            start_time: Instant::now(),
            semaphore: Arc::new(Semaphore::new(100)),
            http: client,
        }
    }
}
