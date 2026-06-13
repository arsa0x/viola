use async_trait::async_trait;
// use wacore::net::StreamingHttpResponse;
use whatsapp_rust::http::{HttpClient, HttpRequest, HttpResponse};

pub struct ReqwestHttpClient {
    pub client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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
        let body = response.bytes().await?;

        Ok(HttpResponse {
            status_code: status.as_u16(),
            body,
        })
    }

    // fn supports_streaming(&self) -> bool {
    //     true
    // }

    // fn execute_streaming(
    //     &self,
    //     request: HttpRequest,
    // ) -> Result<StreamingHttpResponse, anyhow::Error> {
    //     Ok(StreamingHttpResponse {
    //         status_code: response.status_code,
    //         body: Box::new(response.body_reader),
    //     })
    // }
}
