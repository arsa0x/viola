use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use url::Url;
use whatsapp_rust::anyhow::{self, anyhow};

static M3U8_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"file\s*:\s*"([^"]+\.m3u8[^"]*)""#).unwrap());
static THUMB_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"image\s*:\s*"([^"]+)""#).unwrap());
static DURATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"duration\s*:\s*"([\d\.]+)""#).unwrap());
static RESOLUTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"RESOLUTION=(\d+x\d+)"#).unwrap());
static BANDWIDTH_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"BANDWIDTH=(\d+)"#).unwrap());

#[derive(Debug)]
pub struct Stream {
    pub url: String,
    pub quality: Option<String>,
    pub codecs: Option<String>,
    pub bandwidth: Option<i64>,
    pub size: Option<i64>,
    pub mime_type: Option<String>,
}

#[viola_macros::command(
  triggers = ["streampoi", "sp"],
  category = "resolver"
)]
async fn streampoi(ctx: viola_core::Context) -> anyhow::Result<()> {
    let Some(url) = ctx.args.iter().find(|arg| arg.starts_with("https://")) else {
        return ctx.send().text("url-nya mana cik").await;
    };

    let parsed_url = Url::parse(url)?;
    const UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:152.0) Gecko/20100101 Firefox/152.0";

    let response = ctx
        .http_client
        .get(parsed_url.to_string())
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await?;

    let html = response.text().await?;
    let (title, packed_script) = {
        let document = Html::parse_document(&html);

        let title = document
            .select(&Selector::parse("title").map_err(|_| anyhow!("invalid title selector"))?)
            .next()
            .map(|node| node.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let selector = Selector::parse("script").map_err(|_| anyhow!("invalid selector"))?;

        let packed_script = document
            .select(&selector)
            .filter_map(|node| {
                let text = node.text().collect::<String>();
                if text.contains("function(p,a,c,k,e,d)") {
                    Some(text)
                } else {
                    None
                }
            })
            .next()
            .ok_or_else(|| anyhow!("packed script not found"))?;

        (title, packed_script)
    };

    let unpacked = unpack(&packed_script)?;

    let thumbnail = extract_thumbnail(&unpacked);
    let duration = extract_duration(&unpacked);
    let playlist_url = extract_m3u8(&unpacked).ok_or_else(|| anyhow!("m3u8 not found"))?;

    let m3u8_response = ctx
        .http_client
        .get(&playlist_url)
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await?;

    let m3u8_raw = m3u8_response.text().await?;
    let streams = parse_m3u8(&m3u8_raw, &playlist_url);

    let message = format!(
        "Title: {}\n\
         Thumbnail: {:?}\n\
         Duration: {:?}\n\
         Playlist: {}\n\n\
         Streams:\n{:#?}",
        title, thumbnail, duration, playlist_url, streams,
    );

    ctx.send().inapp_signup(message).await
}

fn extract_m3u8(script: &str) -> Option<String> {
    M3U8_REGEX
        .captures(script)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_thumbnail(script: &str) -> Option<String> {
    THUMB_REGEX
        .captures(script)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_duration(script: &str) -> Option<i64> {
    let value = DURATION_REGEX
        .captures(script)?
        .get(1)?
        .as_str()
        .parse::<f64>()
        .ok()?;
    Some(value as i64)
}

fn parse_m3u8(raw: &str, base_url: &str) -> Vec<Stream> {
    let mut streams = Vec::new();
    let mut current_quality = None;
    let mut current_bandwidth = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXT-X-STREAM-INF") {
            let resolution = RESOLUTION_REGEX
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            let bandwidth = BANDWIDTH_REGEX
                .captures(line)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse().ok());

            current_quality = resolution;
            current_bandwidth = bandwidth;
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            let stream_url = if line.starts_with("http://") || line.starts_with("https://") {
                line.to_string()
            } else if let Ok(base) = Url::parse(base_url) {
                base.join(line)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| line.to_string())
            } else {
                line.to_string()
            };

            streams.push(Stream {
                url: stream_url,
                quality: current_quality.take(),
                codecs: None,
                bandwidth: current_bandwidth.take(),
                size: None,
                mime_type: Some("application/x-mpegURL".into()),
            });
        }
    }
    streams
}

pub fn unpack(source: &str) -> anyhow::Result<String> {
    let re = Regex::new(r"}\s*\(\s*'(.*)'\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*'(.*)'\.split\('\|'\)")?;

    let captures = re
        .captures(source)
        .ok_or_else(|| anyhow!("invalid packer format: header match failed"))?;

    let payload = captures[1].to_string();
    let radix: u32 = captures[2].parse()?;
    let count: usize = captures[3].parse()?;
    let symtab: Vec<&str> = captures[4].split('|').collect();

    if symtab.len() != count {
        return Err(anyhow!(
            "malformed packer data: expected {} elements, found {}",
            count,
            symtab.len()
        ));
    }

    let word_re = Regex::new(r"\b\w+\b")?;
    let result = word_re.replace_all(&payload, |caps: &regex::Captures| {
        let word = &caps[0];
        let index = usize::from_str_radix(word, radix).ok();
        match index {
            Some(i) if i < symtab.len() && !symtab[i].is_empty() => symtab[i].to_string(),
            _ => word.to_string(),
        }
    });

    Ok(result.into_owned())
}
