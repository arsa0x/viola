use image::{RgbaImage, imageops::overlay, load_from_memory};
use viola_core::context::Context;
use viola_macros::command;
use webp::Encoder;

#[command(trigger = ["sticker", "s"])]
async fn sticker(ctx: Context) -> anyhow::Result<()> {
    if let Some(img_msg) = &ctx.msg.message.image_message {
        let webp = {
            let bytes = ctx.msg.client.download(img_msg.as_ref()).await?;
            let img = load_from_memory(&bytes)?;
            let resized = img.thumbnail(512, 512);

            let mut canvas = RgbaImage::new(512, 512);

            let x = (512 - resized.width()) / 2;
            let y = (512 - resized.height()) / 2;

            overlay(&mut canvas, &resized.to_rgba8(), x.into(), y.into());

            let encoder = Encoder::from_rgba(canvas.as_raw(), canvas.width(), canvas.height());

            encoder.encode(75.0).to_vec()
        };

        ctx.reply_media(
            viola_core::context::MediaSource::Bytes(webp),
            wacore::download::MediaType::Sticker,
            None,
        )
        .await?;
    }

    Ok(())
}
