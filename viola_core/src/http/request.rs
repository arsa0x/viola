use crate::context::Context;
use isahc::{AsyncBody, Response};
use std::{collections::HashMap, future::Future, pin::Pin};

pub struct HttpRequestBuilder<'a> {
    pub ctx: &'a Context,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl<'a> HttpRequestBuilder<'a> {
    pub fn new(ctx: &'a Context, method: String, url: String) -> Self {
        Self {
            ctx,
            method,
            url,
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

    pub fn json<T: serde::Serialize>(mut self, value: &T) -> anyhow::Result<Self> {
        self.headers
            .insert("Content-Type".into(), "application/json".into());

        self.body = Some(serde_json::to_string(value)?);

        Ok(self)
    }

    pub async fn send(self) -> anyhow::Result<Response<AsyncBody>> {
        let mut builder = isahc::Request::builder()
            .method(self.method.as_str())
            .uri(self.url.as_str());

        for (k, v) in self.headers {
            builder = builder.header(k, v);
        }

        let request = builder.body(self.body.unwrap_or_default())?;

        Ok(self.ctx.state.http.send_async(request).await?)
    }
}

impl<'a> IntoFuture for HttpRequestBuilder<'a> {
    type Output = anyhow::Result<Response<AsyncBody>>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
