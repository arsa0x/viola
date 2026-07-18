use image::{RgbaImage, imageops::overlay, load_from_memory};
use viola_core::context::Context;
use viola_macros::command;
use webp::Encoder;
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
                let webp: Vec<u8> = {
                    let img = load_from_memory(&bytes)?;
                    let resized = img.thumbnail(512, 512);

                    let mut canvas = RgbaImage::new(512, 512);

                    let x = (512 - resized.width()) / 2;
                    let y = (512 - resized.height()) / 2;

                    overlay(&mut canvas, &resized.to_rgba8(), x.into(), y.into());

                    let encoder =
                        Encoder::from_rgba(canvas.as_raw(), canvas.width(), canvas.height());

                    let webp_memory = encoder.encode(75.0);

                    webp_memory.to_vec()
                };

                ctx.send().sticker(webp).quoted().await?;
            }
            _ => {
                ctx.send().failed().await?;
            }
        }
    }
    Ok(())
}
