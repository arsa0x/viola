use crate::framework::context::Context;
use base64::{Engine as _, engine::general_purpose};
use futures::AsyncReadExt;
use isahc::{get_async, http::request::Builder, prelude::*};
use macros::command;
use serde::Deserialize;
use url::Url;
use waproto::whatsapp::{
    self,
    message::{AudioMessage, VideoMessage},
};

/*
 * Name: Tiktok Downloader
 * Creator: Ryza
 * Base: https://www.tiktokdl.web.id
 * Code: https://gist.github.com/Ditzzx-vibecoder/a02d67693ae9216d4c314f0e191c5b4f
 * Sumber: https://whatsapp.com/channel/0029VbCHRSDAzNboLatr0W0o
 * Note: -
 */

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
        ctx.reply("usage: .tiktok <tiktok_url>").await?;
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
                ctx.reply(&format!("failed\ntime: {}ms", ctx.elapsed_ms()))
                    .await?;
            } else {
                let url = if audio_only {
                    let bytes = general_purpose::STANDARD.decode(result.audio_id)?;
                    String::from_utf8(bytes).expect("Invalid UTF-8")
                } else {
                    let bytes = general_purpose::STANDARD.decode(result.video_id)?;
                    String::from_utf8(bytes).expect("Invalid UTF-8")
                };

                let mut resp = get_async(url).await?;
                let mut media_bytes = Vec::new();
                resp.body_mut().read_to_end(&mut media_bytes).await?;
                let mtype = if audio_only {
                    wacore::download::MediaType::Audio
                } else {
                    wacore::download::MediaType::Video
                };

                let len = &media_bytes.len();
                let upload = ctx
                    .msg
                    .client
                    .upload(media_bytes, mtype, Default::default())
                    .await?;

                let ctx_info = ctx.msg.build_quote_context();

                let reply = if audio_only {
                    whatsapp::Message {
                        audio_message: Some(Box::new(AudioMessage {
                            url: Some(upload.url.clone()),
                            file_sha256: Some(upload.file_sha256_vec()),
                            file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                            media_key: Some(upload.media_key_vec()),
                            mimetype: Some("audio/mpeg".to_string()),
                            direct_path: Some(upload.direct_path.clone()),
                            file_length: Some(*len as u64),
                            context_info: Some(Box::new(ctx_info)),

                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                } else {
                    whatsapp::Message {
                        video_message: Some(Box::new(VideoMessage {
                            url: Some(upload.url.clone()),
                            file_sha256: Some(upload.file_sha256_vec()),
                            file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                            media_key: Some(upload.media_key_vec()),
                            mimetype: Some("video/mp4".to_string()),
                            direct_path: Some(upload.direct_path.clone()),
                            file_length: Some(*len as u64),
                            context_info: Some(Box::new(ctx_info)),
                            caption: Some(format!(
                                "author: {}\ntime: {}ms",
                                result.author,
                                ctx.elapsed_ms()
                            )),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                };

                if let Err(e) = ctx.msg.send_message(reply).await {
                    log::error!("failed to send message: {}", e);
                }
                if audio_only {
                    ctx.reply(&format!(
                        "author: {}\ntime: {}ms",
                        result.author,
                        ctx.elapsed_ms()
                    ))
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
