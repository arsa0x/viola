use crate::{config::Config, router::Router};
use isahc::AsyncBody;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc, time::Instant};
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

    pub async fn read_config(&self) -> Config {
        self.config.read().await.clone()
    }
    pub fn save_config(&self, config: &Config) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(config)?;
        fs::write(self.dir.join("config.toml"), content)?;
        Ok(())
    }

    pub fn request(&self, method: impl Into<String>, url: impl Into<String>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(method, url)
    }

    pub async fn send(
        &self,
        req: HttpRequestBuilder,
    ) -> anyhow::Result<isahc::Response<AsyncBody>> {
        let mut builder = isahc::Request::builder()
            .method(req.method.as_str())
            .uri(req.url.as_str());

        for (key, value) in req.headers {
            builder = builder.header(key, value);
        }
        let request = builder.body(req.body.unwrap_or_default())?;

        Ok(self.http.send_async(request).await?)
    }
}

pub struct HttpRequestBuilder {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl HttpRequestBuilder {
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}
