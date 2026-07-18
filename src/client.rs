use isahc::{AsyncBody, AsyncReadResponseExt, Body, Request};
use std::io::Read;
use std::sync::Mutex;
use whatsapp_rust::{
    anyhow, async_trait,
    http::{HttpClient, HttpRequest, HttpResponse},
    wacore::net::{StreamingHttpResponse, UploadBody},
};

pub struct IsahcClient {
    pub client: isahc::HttpClient,
}

impl IsahcClient {
    pub fn new() -> Self {
        Self {
            client: isahc::HttpClient::builder()
                .build()
                .expect("failed to build http client"),
        }
    }
}

struct ThreadSafeReader {
    inner: Mutex<UploadBody>,
}

impl std::io::Read for ThreadSafeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        guard.read(buf)
    }
}

#[async_trait]
impl HttpClient for IsahcClient {
    /// Executes a given HTTP request and returns the response.
    async fn execute(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let method = isahc::http::Method::from_bytes(request.method.as_bytes())?;
        let mut req_builder = isahc::Request::builder().method(&method).uri(request.url);

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        let body_content = request.body.unwrap_or_default();
        let req: Request<AsyncBody> =
            req_builder.body(AsyncBody::from_bytes_static(body_content))?;

        let mut response = self.client.send_async(req).await?;

        let body = response.bytes().await?;
        let status_code = response.status().as_u16();

        Ok(HttpResponse { status_code, body })
    }

    /// Whether this client supports synchronous streaming downloads.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Synchronous streaming variant — returns a reader over the response body.
    /// Must be called from a blocking context.
    fn execute_streaming(&self, request: HttpRequest) -> anyhow::Result<StreamingHttpResponse> {
        let method = isahc::http::Method::from_bytes(request.method.as_bytes())?;
        let mut req_builder = isahc::Request::builder().method(method).uri(request.url);

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        let body_content = request.body.unwrap_or_default();
        let req: Request<Body> = req_builder.body(Body::from_bytes_static(body_content))?;

        let response = self.client.send(req)?;
        let status_code = response.status().as_u16();

        Ok(StreamingHttpResponse {
            status_code,
            body: Box::new(response.into_body()),
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
        let method = isahc::http::Method::from_bytes(request.method.as_bytes())?;
        let mut req_builder = isahc::Request::builder().method(method).uri(request.url);

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        req_builder = req_builder.header("Content-Length", content_length.to_string());

        let reader_proxy = ThreadSafeReader {
            inner: Mutex::new(body),
        };

        let req_body = Body::from_reader_sized(reader_proxy, content_length);
        let req: Request<Body> = req_builder.body(req_body)?;

        let mut response = self.client.send(req)?;
        let status_code = response.status().as_u16();
        let mut resp_body = Vec::new();
        response.body_mut().read_to_end(&mut resp_body)?;

        Ok(HttpResponse {
            status_code,
            body: resp_body,
        })
    }
}
