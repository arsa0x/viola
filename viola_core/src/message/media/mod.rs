pub mod audio;
pub mod document;
pub mod sticker;
pub mod image;
pub mod video;

use isahc::AsyncReadResponseExt;
use whatsapp_rust::anyhow;

use crate::Context;

pub enum MediaSource<'a> {
    Url(&'a str),
    Bytes(Vec<u8>),
}

impl<'a> MediaSource<'a> {
    pub async fn get_media_bytes(self, ctx: &'a Context) -> anyhow::Result<Vec<u8>> {
        match self {
            MediaSource::Bytes(b) => Ok(b),
            MediaSource::Url(url) => {
                let mut response = ctx.http_client.get_async(url).await?;
                let bytes = response.bytes().await?;
                Ok(bytes)
            }
        }
    }
}
