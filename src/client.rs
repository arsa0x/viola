use std::io::Read;

use bytes::Bytes;
use whatsapp_rust::{
    HttpResourceReport, anyhow, async_trait,
    http::{HttpClient, HttpRequest, HttpResponse},
    wacore::net::{StreamingHttpResponse, UploadBody},
};

pub const DEFAULT_MAX_BODY_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB
pub const ERROR_BODY_CAP: u64 = 64 * 1024;
pub const UPLOAD_CHUNK_BYTES: usize = 64 * 1024;
pub const UPLOAD_CHANNEL_CAPACITY: usize = 4;

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
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }

    #[allow(unused)]
    pub fn with_max_body_bytes(mut self, max_body_bytes: u64) -> Self {
        self.max_body_bytes = max_body_bytes;
        self
    }
}

async fn read_body_capped(
    mut res: reqwest::Response,
    max_body_bytes: u64,
) -> anyhow::Result<Vec<u8>> {
    let is_success = res.status().is_success();
    let cap = if is_success {
        max_body_bytes
    } else {
        max_body_bytes.min(ERROR_BODY_CAP)
    };

    let content_length = res.content_length().unwrap_or(0).min(cap) as usize;
    let mut body = Vec::with_capacity(content_length);

    while let Some(chunk) = res.chunk().await? {
        if body.len() as u64 + chunk.len() as u64 > cap {
            if is_success {
                anyhow::bail!("response body exceeds max_body_bytes cap");
            }

            break;
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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

        let body = read_body_capped(res, self.max_body_bytes).await?;

        Ok(HttpResponse { status_code, body })
    }

    /// Whether this client supports synchronous streaming downloads.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Synchronous streaming variant — returns a reader over the response body.
    /// Must be called from a blocking context.
    fn execute_streaming(&self, request: HttpRequest) -> anyhow::Result<StreamingHttpResponse> {
        if request.method != "GET" {
            anyhow::bail!("Streaming only supports GET, got: {}", request.method);
        }

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
        if request.method != "POST" {
            anyhow::bail!(
                "Upload streaming only supports POST, got: {}",
                request.method
            );
        }

        let handle = tokio::runtime::Handle::current();
        let client = self.client.clone();
        let url = request.url.clone();
        let headers = request.headers.clone();
        let max_body_bytes = self.max_body_bytes;

        handle.block_on(async move {
            let (tx, rx) =
                tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(UPLOAD_CHANNEL_CAPACITY);

            tokio::task::spawn_blocking(move || {
                let mut reader = body;
                let mut buf = vec![0u8; UPLOAD_CHUNK_BYTES];
                let mut total_read = 0u64;

                loop {
                    let n = match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(e));
                            return;
                        }
                    };

                    total_read += n as u64;

                    if total_read > content_length {
                        let _ = tx.blocking_send(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "upload body exceeds declared Content-Length",
                        )));
                        return;
                    }

                    if tx
                        .blocking_send(Ok(Bytes::copy_from_slice(&buf[..n])))
                        .is_err()
                    {
                        return;
                    }
                }

                if total_read != content_length {
                    let _ = tx.blocking_send(Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!(
                            "upload body length mismatch: expected {}, got {}",
                            content_length, total_read
                        ),
                    )));
                }
            });

            let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
            let body = reqwest::Body::wrap_stream(stream);
            let mut req = client.post(&url);

            for (key, value) in &headers {
                req = req.header(key, value);
            }

            req = req.header(reqwest::header::CONTENT_LENGTH, content_length);
            req = req.body(body);

            let res = req.send().await?;
            let status_code = res.status().as_u16();
            let body = read_body_capped(res, max_body_bytes).await?;

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
