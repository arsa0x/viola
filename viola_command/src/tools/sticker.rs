use image::{
    ImageEncoder, RgbaImage, codecs::webp::WebPEncoder, imageops::overlay, load_from_memory,
};
use viola_core::{context::Context, message::media::MediaSource};
use viola_macros::command;
use whatsapp_rust::{anyhow, download::MediaType, serde_json::json};

const DEFAULT_PUBLISHER: &str = "arsa";
const DEFAULT_NAME: &str = "github: arsa0x/viola";

#[command(
    triggers = ["sticker", "stiker", "s"],
    category = "tools",
    description = "Convert image to whatsapp sticker"
)]
async fn sticker(ctx: Context) -> anyhow::Result<()> {
    let metadata = create_whatsapp_exif(None, None);

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

                let mut webp_buffer = Vec::new();
                let mut encoder = WebPEncoder::new_lossless(&mut webp_buffer);

                encoder.set_exif_metadata(metadata)?;
                canvas.write_with_encoder(encoder)?;

                ctx.send()
                    .sticker(MediaSource::Bytes(webp_buffer))
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

fn create_whatsapp_exif(name: Option<&str>, publisher: Option<&str>) -> Vec<u8> {
    let publisher = if let Some(p) = publisher {
        p
    } else {
        DEFAULT_PUBLISHER
    };
    let pack_name = if let Some(n) = name { n } else { DEFAULT_NAME };

    let json_meta = json!({
        "sticker-pack-id": "viola_bot",
        "sticker-pack-name": pack_name,
        "sticker-pack-publisher": publisher,
    });

    let json_string = json_meta.to_string();
    let json_bytes = json_string.as_bytes();
    let json_len = json_bytes.len() as u32;

    let mut exif_bytes = vec![
        0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x41, 0x57, 0x07, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
    ];

    let len_bytes = json_len.to_le_bytes();
    exif_bytes[14..18].copy_from_slice(&len_bytes);
    exif_bytes.extend_from_slice(&json_bytes);
    exif_bytes
}
