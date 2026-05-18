use isahc::{Request, prelude::*};
use std::collections::HashMap;
use url::Url;
use viola_core::framework::context::Context;
use viola_macros::command;

const HELP: &str = r#"USAGE:
  .http <METHOD> <URL> [OPTIONS]

METHODS:
  GET
  POST
  PUT
  PATCH
  DELETE

OPTIONS:
  -h, --header "<KEY>: <VALUE>"
      Add request header

  -q, --query "<KEY>=<VALUE>"
      Add query parameter

  -d, --data "<BODY>"
      Send request body

EXAMPLES:

  Simple GET request
    .http GET https://httpbin.org/get

  GET with query params
    .http GET https://example.com \
      -q "page=1" \
      -q "limit=10"

  POST JSON request
    .http POST https://httpbin.org/post \
      -h "Content-Type: application/json" \
      -d "{\"name\":\"john\"}"

  Custom Authorization header
    .http GET https://api.example.com/me \
      -h "Authorization: Bearer token""#;

#[command(trigger = ["http", "https", "fetch"], help = HELP, description = "Send HTTP requests")]
async fn http_request(ctx: Context) -> anyhow::Result<()> {
    if ctx.args.len() < 2 {
        ctx.reply(
            "usage:\n.http GET http://example.com \n-h \"Content-Type: application/json\" \n-q \"page=1\"",
        )
        .await?;
        return Ok(());
    }

    let method = ctx.args[0].to_uppercase();
    let mut url_str = ctx.args[1].clone();

    let mut headers = HashMap::new();
    let mut queries = Vec::new();

    let mut i = 2;
    while i < ctx.args.len() {
        match ctx.args[i].as_str() {
            "-h" | "--header" => {
                if i + 1 < ctx.args.len() {
                    let val = &ctx.args[i + 1];

                    if let Some((key, value)) = val.split_once(':') {
                        headers.insert(key.trim().to_string(), value.trim().to_string());
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-q" | "--query" => {
                if i + 1 < ctx.args.len() {
                    let val = &ctx.args[i + 1];
                    if let Some((key, value)) = val.split_once('=') {
                        queries.push((key.trim().to_string(), value.trim().to_string()));
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-d" | "--data" => {
                // to do
            }
            _ => {
                i += 1;
            }
        }
    }

    if !queries.is_empty() {
        if let Ok(mut url_obj) = Url::parse(&url_str) {
            {
                let mut query_pairs = url_obj.query_pairs_mut();
                for (k, v) in queries {
                    query_pairs.append_pair(&k, &v);
                }
            }
            url_str = url_obj.to_string();
        }
    }

    let mut builder = Request::builder().method(method.as_str()).uri(&url_str);

    for (key, val) in headers {
        builder = builder.header(&key, &val);
    }

    let request = match builder.body(()) {
        Ok(req) => req,
        Err(e) => {
            ctx.reply(&e.to_string()).await?;
            return Ok(());
        }
    };

    let client = isahc::HttpClient::new()?;

    match client.send_async(request).await {
        Ok(mut res) => {
            let status = res.status();
            let body_text = res
                .text()
                .await
                .unwrap_or_else(|_| "failed to read body".to_string());

            let trimmed_body = if body_text.len() > 1500 {
                format!("{}...", &body_text[..1500])
            } else {
                body_text
            };

            ctx.reply(&format!(
                "status: {}\n\ntime: {}ms\n\nbody:\n```{}\n```",
                status,
                ctx.elapsed_ms(),
                trimmed_body
            ))
            .await?;
        }
        Err(e) => {
            ctx.reply(&format!("request failed: {}", e)).await?;
        }
    }

    Ok(())
}
