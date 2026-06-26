use super::request::HttpRequestBuilder;
use crate::context::Context;

pub struct HttpGetBuilder<'a> {
    inner: HttpRequestBuilder<'a>,
}

impl<'a> HttpGetBuilder<'a> {
    pub fn new(ctx: &'a Context, url: String) -> Self {
        Self {
            inner: HttpRequestBuilder::new(ctx, "GET".into(), url),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.header(key, value);
        self
    }

    pub async fn send(self) -> anyhow::Result<isahc::Response<isahc::AsyncBody>> {
        self.inner.send().await
    }
}
