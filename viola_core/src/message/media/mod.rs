pub mod audio;
pub mod document;
pub mod image;
pub mod sticker;
pub mod video;

use ::image::{GenericImageView, codecs::jpeg::JpegEncoder, imageops::FilterType};
use whatsapp_rust::anyhow;

use crate::Context;

pub enum MediaSource {
    Url(String),
    Bytes(Vec<u8>),
}

impl<'a> MediaSource {
    pub async fn get_media_bytes(self, ctx: &'a Context) -> anyhow::Result<Vec<u8>> {
        match self {
            MediaSource::Bytes(b) => Ok(b),
            MediaSource::Url(url) => {
                let response = ctx.http_client.get(&url).send().await?;
                let bytes = response.bytes().await?;
                Ok(bytes.to_vec())
            }
        }
    }
}

pub const DEFAULT_MAX_DIM: u32 = 200;
pub const DEFAULT_JPEG_QUALITY: u8 = 60;

pub fn image_thumbnail(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    image_thumbnail_with(bytes, DEFAULT_MAX_DIM, DEFAULT_JPEG_QUALITY)
}

pub fn image_thumbnail_with(bytes: &[u8], max_dim: u32, quality: u8) -> anyhow::Result<Vec<u8>> {
    let img = ::image::load_from_memory(bytes)?;
    let (w, h) = img.dimensions();

    let resized = if w <= max_dim && h <= max_dim {
        img
    } else {
        img.resize(max_dim, max_dim, FilterType::Triangle)
    };

    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality).encode_image(&resized)?;
    Ok(out)
}

pub async fn image_thumbnail_async(bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || image_thumbnail(&bytes)).await?
}

pub async fn image_thumbnail_async_with(
    bytes: Vec<u8>,
    max_dim: u32,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || image_thumbnail_with(&bytes, max_dim, quality)).await?
}
