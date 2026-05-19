/*
 * Name: Tiktok Downloader
 * Creator: Ryza
 * Base: https://www.tiktokdl.web.id
 * Code: https://gist.github.com/Ditzzx-vibecoder/a02d67693ae9216d4c314f0e191c5b4f
 * Sumber: https://whatsapp.com/channel/0029VbCHRSDAzNboLatr0W0o
 * Note: -
 */

use base64::{Engine as _, engine::general_purpose};
use isahc::{http::request::Builder, prelude::*};
use serde::Deserialize;
use url::Url;
use viola_core::context::{Context, MediaSource};
use viola_macros::command;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TikTokData {
    pub status: bool,
    pub author: String,
    pub video_id: String,
    pub audio_id: String,
}

#[command(trigger = ["tt", "tiktok", "tik"])]
async fn tiktok(ctx: Context) -> anyhow::Result<()> {
    let Some(tiktok_url) = ctx.args.iter().find(|f| f.contains("https")) else {
        ctx.reply("usage: .tiktok [-mp3] <tiktok_url>").await?;
        return Ok(());
    };

    let audio_only = ctx.args.iter().any(|f| {
        f.contains("--mp3") || f.contains("--audio") || f.contains("-mp3") || f.contains("-audio")
    });

    let mut uri = String::new();
    if let Ok(mut url_obj) = Url::parse("https://www.tiktokdl.web.id/api/tiktok") {
        url_obj.query_pairs_mut().append_pair("url", tiktok_url);
        uri = url_obj.to_string();
    }

    let request = Builder::new()
        .method("GET")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .uri(&uri)
        .body(())?;

    match ctx.state.http.send_async(request).await {
        Ok(mut response) => {
            let res = response.text().await?;

            let result: TikTokData = serde_json::from_str(&res)?;

            if !result.status {
                ctx.reply(&format!("failed\ntime: {:.3}ms", ctx.elapsed_ms_f64()))
                    .await?;
            } else {
                if audio_only {
                    let bytes = general_purpose::STANDARD.decode(result.audio_id)?;
                    ctx.reply_media(
                        MediaSource::Url(String::from_utf8(bytes)?),
                        wacore::download::MediaType::Audio,
                        Some(format!(
                            "author: {}\ntime: {:.3}ms",
                            result.author,
                            ctx.elapsed_ms_f64()
                        )),
                    )
                    .await?;
                } else {
                    let bytes = general_purpose::STANDARD.decode(result.video_id)?;
                    ctx.reply_media(
                        MediaSource::Url(String::from_utf8(bytes)?),
                        wacore::download::MediaType::Video,
                        Some(format!(
                            "author: {}\ntime: {:.3}ms",
                            result.author,
                            ctx.elapsed_ms_f64()
                        )),
                    )
                    .await?;
                }
            }
        }
        Err(e) => {
            ctx.reply(&e.to_string()).await?;
        }
    }

    Ok(())
}
