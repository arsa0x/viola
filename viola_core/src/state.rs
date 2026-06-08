use crate::{config::Config, router::Router};
use reqwest::Client;
use std::{path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::Semaphore;

pub struct AppState {
    pub config: Arc<Config>,
    pub router: Arc<Router>,
    pub start_time: Instant,
    pub semaphore: Arc<Semaphore>,
    pub http: Client,
    pub http_no_redirect: Client,
    pub dir: Arc<PathBuf>,
}

impl AppState {
    pub fn new(config: Arc<Config>, router: Arc<Router>, dir: Arc<PathBuf>) -> Self {
        let http_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("failed to build reqwest client");

        let http_no_redirect = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("failed to build reqwest client with no redirect");

        Self {
            config,
            router,
            start_time: Instant::now(),
            semaphore: Arc::new(Semaphore::new(100)),
            http: http_client,
            http_no_redirect: http_no_redirect,
            dir,
        }
    }
}
