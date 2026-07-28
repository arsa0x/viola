use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use rand::RngExt;
use regex::Regex;
use scraper::{Html, Selector};
use url::Url;
use whatsapp_rust::anyhow::{self, anyhow};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
          (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

static PASS_MD5_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\$\.get\(\s*['"]([^'"]*/pass_md5/[^'"]+)['"]"#).unwrap());

#[viola_macros::command(
  triggers = ["playmogo", "pm"],
  category = "resolver"
)]
async fn playmogo(ctx: viola_core::Context) -> anyhow::Result<()> {
    let Some(url) = ctx.args.iter().find(|arg| arg.starts_with("https://")) else {
        return ctx.send().text("url-nya mana cik").await;
    };

    let mut parsed_url = Url::parse(url)?;

    {
        let mut path_parts: Vec<String> = parsed_url
            .path_segments()
            .map(|s| s.map(String::from).collect())
            .unwrap_or_default();

        if !path_parts.is_empty() {
            path_parts[0] = "e".to_string();

            let mut segments = parsed_url
                .path_segments_mut()
                .map_err(|_| anyhow!("Cannot be base URL"))?;

            segments.clear();
            for part in &path_parts {
                segments.push(part);
            }
        }
    }

    let response = ctx
        .http_client
        .get(parsed_url.clone())
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await?;

    let html = response.text().await?;

    let (title, passmd) = {
        let document = Html::parse_document(&html);

        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|node| node.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let script_selector = Selector::parse("script").map_err(|_| anyhow!("invalid selector"))?;

        let script_text = document
            .select(&script_selector)
            .map(|node| node.text().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        let passmd = PASS_MD5_REGEX
            .captures(&script_text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| anyhow!("pass_md5 endpoint not found"))?;

        (title, passmd)
    };

    let token = passmd
        .split('/')
        .last()
        .ok_or_else(|| anyhow!("token not found in passmd string"))?
        .to_string();

    let origin = parsed_url.origin().ascii_serialization();
    let urlpassmd = format!("{}{}", origin, passmd);

    let resp = ctx
        .http_client
        .get(&urlpassmd)
        .header(reqwest::header::USER_AGENT, UA)
        .header("Referer", url)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await?;

    let urlf = resp.text().await?;

    let final_stream_url = generate_final_url(&urlf, &token);

    ctx.send().inapp_signup(final_stream_url).title(title).await
}

fn generate_final_url(base_url: &str, token: &str) -> String {
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    let random_string: String = (0..10)
        .map(|_| {
            let idx = rng.random_range(0..charset.len());
            charset[idx] as char
        })
        .collect();

    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{}{random_string}?token={token}&expiry={expiry}", base_url)
}
