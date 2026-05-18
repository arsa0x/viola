use crate::{framework::router::Router, utils::config::Config};
use std::{sync::Arc, time::Instant};
use tokio::sync::Semaphore;

pub struct AppState {
    pub config: Arc<Config>,
    pub router: Arc<Router>,
    pub start_time: Instant,
    pub semaphore: Arc<Semaphore>,
    pub http: Arc<isahc::HttpClient>,
}

pub type SharedState = Arc<AppState>;

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
