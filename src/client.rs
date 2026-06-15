use async_trait::async_trait;
use whatsapp_rust::http::{HttpClient, HttpRequest, HttpResponse};

pub struct ReqwestHttpClient {
    pub client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .build()
                .expect("failed to build async client"),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn execute(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let method = request.method.parse::<reqwest::Method>()?;
        let mut req_builder = self.client.request(method, request.url);

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        let response = req_builder.send().await?;
        let status = response.status();
        let body = response.bytes().await?.to_vec();

        Ok(HttpResponse {
            status_code: status.as_u16(),
            body,
        })
    }

    // fn supports_streaming(&self) -> bool {
    //     true
    // }

    // fn execute_streaming(&self, request: HttpRequest) -> anyhow::Result<StreamingHttpResponse> {
    //     let method = request.method.parse::<reqwest::Method>()?;
    //     let client = self.blocking_client.as_ref().unwrap();
    //     let mut req_builder = client.request(method, request.url);

    //     for (key, value) in request.headers {
    //         req_builder = req_builder.header(key, value);
    //     }

    //     if let Some(body) = request.body {
    //         req_builder = req_builder.body(body.to_vec());
    //     }

    //     let response = req_builder.send()?;
    //     let status = response.status();

    //     Ok(StreamingHttpResponse {
    //         status_code: status.as_u16(),
    //         body: Box::new(response),
    //     })
    // }

    // fn supports_upload_streaming(&self) -> bool {
    //     true
    // }

    // fn execute_upload(
    //     &self,
    //     request: HttpRequest,
    //     body: UploadBody,
    //     content_length: u64,
    // ) -> anyhow::Result<HttpResponse> {
    //     let method = request.method.parse::<reqwest::Method>()?;
    //     let client = self.blocking_client.as_ref().unwrap();
    //     let mut req_builder = client.request(method, request.url);

    //     for (key, value) in request.headers {
    //         req_builder = req_builder.header(key, value);
    //     }

    //     req_builder = req_builder.header(reqwest::header::CONTENT_LENGTH, content_length);

    //     let req_body = reqwest::blocking::Body::sized(body, content_length);
    //     req_builder = req_builder.body(req_body);

    //     let response = req_builder.send()?;
    //     let status = response.status();
    //     let resp_body = response.bytes()?.to_vec();

    //     Ok(HttpResponse {
    //         status_code: status.as_u16(),
    //         body: resp_body,
    //     })
    // }
}
