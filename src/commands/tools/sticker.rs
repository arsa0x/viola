use image::load_from_memory;
use macros::command;
use waproto::whatsapp::{self, message::StickerMessage};
use webp::Encoder;

use crate::framework::context::Context;

#[command(trigger = ["sticker", "s"])]
async fn sticker(ctx: Context) -> anyhow::Result<()> {
    if let Some(img_msg) = &ctx.msg.message.image_message {
        let webp = {
            let img = load_from_memory(&ctx.msg.client.download(img_msg.as_ref()).await?)?;
            let resized = img.thumbnail(512, 512);
            let rgba = resized.to_rgba8();
            let encoder = Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
            encoder.encode(75.0).to_vec()
        };
        let upload = ctx
            .msg
            .client
            .upload(
                webp.to_vec(),
                wacore::download::MediaType::Sticker,
                Default::default(),
            )
            .await?;
        let ctx_info = ctx.msg.build_quote_context();
        let reply = whatsapp::Message {
            sticker_message: Some(Box::new(StickerMessage {
                url: Some(upload.url.clone()),
                file_sha256: Some(upload.file_sha256_vec()),
                file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                media_key: Some(upload.media_key_vec()),
                mimetype: Some("image/webp".to_string()),
                direct_path: Some(upload.direct_path.clone()),
                file_length: Some(webp.len() as u64),
                context_info: Some(Box::new(ctx_info)),
                ..Default::default()
            })),
            ..Default::default()
        };
        if let Err(e) = ctx.msg.send_message(reply).await {
            log::error!("failed to send message: {}", e);
        }
    }
    Ok(())
}
