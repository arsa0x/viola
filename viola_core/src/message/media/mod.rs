use isahc::AsyncReadResponseExt;
use whatsapp_rust::anyhow;

use crate::Context;

pub mod audio;
pub mod document;
pub mod image;
pub mod sticker;
pub mod video;

pub enum MediaSource<'a> {
    Url(&'a str),
    Bytes(Vec<u8>),
}

impl<'a> MediaSource<'a> {
    pub async fn get_media(self, ctx: &'a Context) -> anyhow::Result<Vec<u8>> {
        match self {
            MediaSource::Bytes(b) => Ok(b),
            MediaSource::Url(url) => {
                let mut response = ctx.http_client.get_async(url).await?;
                let b = response.bytes().await?;
                Ok(b)
            }
        }
    }
}
