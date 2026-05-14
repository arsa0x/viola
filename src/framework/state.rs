use moka::future::Cache;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// to odo
pub struct AppState {
    pub cooldowns: Cache<String, bool>,
    pub limiter: Arc<Semaphore>,
}
