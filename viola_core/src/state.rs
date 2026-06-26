use crate::{
    config::{BotMode, Config},
    router::Router,
};
use std::{fs, path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::{RwLock, Semaphore};

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub router: Arc<Router>,
    pub start_time: Instant,
    pub semaphore: Arc<Semaphore>,
    pub http: isahc::HttpClient,
    pub dir: Arc<PathBuf>,
}

impl AppState {
    pub fn new(
        config: Arc<RwLock<Config>>,
        router: Arc<Router>,
        dir: Arc<PathBuf>,
        client: isahc::HttpClient,
    ) -> Self {
        Self {
            config,
            router,
            start_time: Instant::now(),
            semaphore: Arc::new(Semaphore::new(100)),
            dir,
            http: client,
        }
    }

    pub async fn set_bot_mode(&self, mode: BotMode) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.bot.mode = mode;
        let content = toml::to_string_pretty(&*config)?;
        fs::write(self.dir.join("config.toml"), content)?;
        Ok(())
    }

    pub async fn read_config(&self) -> Config {
        self.config.read().await.clone()
    }

    pub async fn save_config(&self, config: Config) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(&config)?;
        fs::write(self.dir.join("config.toml"), content)?;
        *self.config.write().await = config;
        Ok(())
    }
}
