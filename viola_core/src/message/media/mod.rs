pub mod audio;
pub mod document;
pub mod image;
pub mod sticker;
pub mod video;

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
                let response = ctx.http_client.get(url).send().await?;
                let bytes = response.bytes().await?;
                Ok(bytes.to_vec())
            }
        }
    }
}
