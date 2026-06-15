use bytes::Bytes;
use image::{RgbaImage, imageops::overlay, load_from_memory};
use viola_core::context::{Context, media::Thumbnail, message::MessageBuilder};
use viola_macros::command;
use webp::Encoder;
use whatsapp_rust::download::MediaType;

#[command(trigger = ["sticker", "s"])]
async fn sticker(ctx: Context) -> anyhow::Result<()> {
    if let Ok((img_msg, media_type)) = ctx.get_media() {
        if media_type == MediaType::Image {
            let webp: Bytes = {
                let bytes = ctx.msg_ctx.client.download(img_msg).await?;
                let img = load_from_memory(&bytes)?;
                let resized = img.thumbnail(512, 512);

                let mut canvas = RgbaImage::new(512, 512);

                let x = (512 - resized.width()) / 2;
                let y = (512 - resized.height()) / 2;

                overlay(&mut canvas, &resized.to_rgba8(), x.into(), y.into());

                let encoder = Encoder::from_rgba(canvas.as_raw(), canvas.width(), canvas.height());

                let webp_memory = encoder.encode(75.0);

                Bytes::copy_from_slice(&webp_memory)
            };

            let upload = ctx
                .msg_ctx
                .client
                .upload(webp.to_vec(), MediaType::Sticker, Default::default())
                .await?;

            let msg = MessageBuilder {
                ctx: &ctx,
                length: webp.len() as u64,
                upload,
            };

            let thumbnail_bytes = Thumbnail::image_thumbnail_from_memory(&webp);

            ctx.reply(msg.sticker_message(thumbnail_bytes)).await?;
        } else {
            ctx.reply_failed().await?;
        }
    }

    Ok(())
}
