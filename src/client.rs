use std::io::Read;

use bytes::Bytes;
use futures_util::StreamExt;
use whatsapp_rust::{
    HttpResourceReport, anyhow, async_trait,
    http::{HttpClient, HttpRequest, HttpResponse},
    wacore::net::{StreamingHttpResponse, UploadBody},
};

pub const DEFAULT_MAX_BODY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub struct ReqwestClient {
    client: reqwest::Client,
    max_body_bytes: u64,
}

struct BlockingBodyReader {
    handle: tokio::runtime::Handle,
    resp: reqwest::Response,
    buf: Bytes,
    pos: usize,
}

impl Read for BlockingBodyReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.pos < self.buf.len() {
                let n = out.len().min(self.buf.len() - self.pos);
                out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
                self.pos += n;
                return Ok(n);
            }
            let chunk = self
                .handle
                .block_on(self.resp.chunk())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            match chunk {
                Some(c) => {
                    self.buf = c;
                    self.pos = 0;
                }
                None => return Ok(0),
            }
        }
    }
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(2)
                .build()
                .expect("failed to build reqwest client"),
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    /// Executes a given HTTP request and returns the response.
    async fn execute(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let mut req = self.client.request(request.method.parse()?, &request.url);
        for (k, v) in &request.headers {
            req = req.header(k, v);
        }
        if let Some(body) = request.body {
            req = req.body(body);
        }
        let res = req.send().await?;
        let status_code = res.status().as_u16();

        let mut body = Vec::new();
        let mut stream = res.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if body.len() as u64 + chunk.len() as u64 > self.max_body_bytes {
                anyhow::bail!("response body exceeds max_body_bytes cap");
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse { status_code, body })
    }

    /// Whether this client supports synchronous streaming downloads.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Synchronous streaming variant — returns a reader over the response body.
    /// Must be called from a blocking context.
    fn execute_streaming(&self, request: HttpRequest) -> anyhow::Result<StreamingHttpResponse> {
        let handle = tokio::runtime::Handle::current();
        let client = self.client.clone();
        let url = request.url.clone();
        let headers = request.headers.clone();

        let resp = handle.block_on(async move {
            let mut req = client.get(&url);
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            req.send().await
        })?;

        let status_code = resp.status().as_u16();
        let reader = BlockingBodyReader {
            handle,
            resp,
            buf: Bytes::new(),
            pos: 0,
        };
        let capped = std::io::Read::take(reader, self.max_body_bytes);
        Ok(StreamingHttpResponse {
            status_code,
            body: Box::new(capped),
        })
    }

    /// Whether this client can stream a request body from a reader (upload).
    fn supports_upload_streaming(&self) -> bool {
        true
    }

    /// Synchronous streaming upload: send `body` (exactly `content_length` bytes)
    /// as the request body. Implementations MUST set an explicit `Content-Length`
    /// rather than chunked transfer-encoding. Any body set on `request` is
    /// ignored. Must be called from a blocking context.
    fn execute_upload(
        &self,
        request: HttpRequest,
        body: UploadBody,
        content_length: u64,
    ) -> anyhow::Result<HttpResponse> {
        let handle = tokio::runtime::Handle::current();
        let client = self.client.clone();
        let url = request.url.clone();
        let headers = request.headers.clone();
        let max_body = self.max_body_bytes;

        let mut buf = Vec::with_capacity(content_length.min(max_body) as usize);
        let mut body = body;
        std::io::Read::read_to_end(&mut body, &mut buf)?;

        handle.block_on(async move {
            let mut req = client.post(&url);
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            req = req.body(reqwest::Body::from(buf));
            let res = req.send().await?;
            let status_code = res.status().as_u16();
            let body = res.bytes().await?.to_vec();
            Ok(HttpResponse { status_code, body })
        })
    }

    /// Best-effort per-session footprint of this client: idle connection-pool
    /// buffers plus any in-flight download/media buffering the impl can see.
    /// `None` by default; `ureq`/`reqwest`-backed clients report what their
    /// (limited) introspection allows. Media downloads are a real transient-RAM
    /// source, so a coarse estimate is still worth reporting.
    fn resource_report(&self) -> Option<HttpResourceReport> {
        None
    }
}
