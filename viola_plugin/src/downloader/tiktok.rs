/*
 * Name: Tiktok Downloader
 * Creator: Ryza
 * Base: https://www.tiktokdl.web.id
 * Code: https://gist.github.com/Ditzzx-vibecoder/a02d67693ae9216d4c314f0e191c5b4f
 * Sumber: https://whatsapp.com/channel/0029VbCHRSDAzNboLatr0W0o
 * Note: -
 */

use base64::{Engine as _, engine::general_purpose};
use isahc::AsyncReadResponseExt;
use serde::Deserialize;
use url::Url;
use viola_core::context::Context;
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
    ctx.message().wait().await?;

    let Some(tiktok_url) = ctx.args.iter().find(|f| f.contains("https")) else {
        ctx.message().failed().await?;
        ctx.message()
            .text("usage: .tiktok [-mp3] <tiktok_url>")
            .quoted()
            .await?;
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

    if let Err(e) = async {
        let req = isahc::Request::get(&uri)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .body(())?;

        let mut response = ctx.state.http.send_async(req).await?;

        let res = response.text().await?;

        let result: TikTokData = serde_json::from_str(&res)?;

        if !result.status {
            ctx.message().text("failed").await?;
            ctx.message().failed().await?;
            return Ok::<(), anyhow::Error>(());
        }

        if audio_only {
            let bytes = general_purpose::STANDARD.decode(result.audio_id)?;
            let url = String::from_utf8(bytes)?;
            let mut response = ctx.state.http.get_async(&url).await?;
            let media = response.bytes().await?;
            ctx.message().audio(media).await?;
            ctx.message().success().await?;
        } else {
            let bytes = general_purpose::STANDARD.decode(result.video_id)?;
            let url = String::from_utf8(bytes)?;
            let mut response = ctx.state.http.get_async(&url).await?;
            let media = response.bytes().await?;
            ctx.message()
                .video(media)
                .caption(format!("author: {}", result.author))
                .await?;
            ctx.message().success().await?;
        }

        Ok(())
    }
    .await
    {
        ctx.message().failed().await?;
        ctx.message().text(&e.to_string()).await?;
    }

    Ok(())
}
