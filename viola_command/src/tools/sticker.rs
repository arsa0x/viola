use image::{ImageFormat, RgbaImage, imageops::overlay, load_from_memory};
use viola_core::{context::Context, message::media::MediaSource};
use viola_macros::command;
use whatsapp_rust::{anyhow, download::MediaType};

#[command(
    triggers = ["sticker", "stiker", "s"],
    category = "tools",
    description = "Convert image to whatsapp sticker"
)]
async fn sticker(ctx: Context) -> anyhow::Result<()> {
    if let Ok((mtype, media)) = ctx.get_current_media().or_else(|_| ctx.get_quoted_media()) {
        match mtype {
            MediaType::Image => {
                let bytes = ctx.wa_client.download(media).await?;

                let img = load_from_memory(&bytes)?;
                let resized = img.thumbnail(512, 512);

                let mut canvas = RgbaImage::new(512, 512);

                let x = (512 - resized.width()) / 2;
                let y = (512 - resized.height()) / 2;

                overlay(&mut canvas, &resized.to_rgba8(), x.into(), y.into());

                let mut webp = std::io::Cursor::new(Vec::new());

                canvas.write_to(&mut webp, ImageFormat::WebP)?;

                ctx.send()
                    .sticker(MediaSource::Bytes(webp.into_inner()))
                    .quoted()
                    .await?;
            }
            _ => {
                ctx.send().failed().await?;
            }
        }
    }
    Ok(())
}
