use image::load_from_memory;
use macros::command;
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
        ctx.reply_media(
            crate::framework::context::MediaSource::Bytes(webp),
            wacore::download::MediaType::Sticker,
            None,
        )
        .await?;
    }
    Ok(())
}
