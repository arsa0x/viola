use whatsapp_rust::{anyhow, serde::Serialize};

use super::request::HttpRequestBuilder;
use crate::context::Context;

pub struct HttpPostBuilder<'a> {
    inner: HttpRequestBuilder<'a>,
}

impl<'a> HttpPostBuilder<'a> {
    pub fn new(ctx: &'a Context, url: String) -> Self {
        Self {
            inner: HttpRequestBuilder::new(ctx, "POST".into(), url),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.header(key, value);
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.inner = self.inner.body(body);
        self
    }

    pub fn json<T: Serialize>(mut self, value: &T) -> anyhow::Result<Self> {
        self.inner = self.inner.json(value)?;
        Ok(self)
    }

    pub async fn send(self) -> anyhow::Result<isahc::Response<isahc::AsyncBody>> {
        self.inner.send().await
    }
}
