use reqwest::{Method, Url};
use std::collections::HashMap;
use viola_core::context::Context;
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
    ctx.reply_wait().await?;

    if ctx.args.len() < 2 {
        ctx.reply_failed().await?;

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
    let mut body = None::<String>;

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
                if i + 1 < ctx.args.len() {
                    body = Some(ctx.args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
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

    let method = match Method::from_bytes(method.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            ctx.reply_failed().await?;
            ctx.reply("invalid http method").await?;
            return Ok(());
        }
    };

    let mut request = ctx.state.http.request(method, &url_str);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    if let Some(body) = body {
        request = request.body(body);
    }

    match request.send().await {
        Ok(res) => {
            let status = res.status();

            let body_text = match res.text().await {
                Ok(text) => text,
                Err(_) => "failed to read body".to_string(),
            };

            // let trimmed_body = if body_text.len() > 1500 {
            //     format!("{}...", &body_text[..1500])
            // } else {
            //     body_text
            // };

            ctx.reply_success().await?;

            ctx.reply(&format!(
                "status: {}\n\nbody:\n```{}\n```",
                status, body_text
            ))
            .await?;
        }
        Err(e) => {
            ctx.reply(&format!("request failed: {}", e)).await?;
            ctx.reply_failed().await?;
        }
    }

    Ok(())
}
