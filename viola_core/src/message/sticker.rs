use crate::context::Context;
use std::pin::Pin;
use whatsapp_rust::{
    download::MediaType,
    waproto::whatsapp::{self, message::StickerMessage},
};

pub struct StickerBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
}

impl<'a> StickerBuilder<'a> {
    pub fn thumbnail(mut self, thumbnail: Vec<u8>) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let quoted = if self.quoted {
            Some(Box::new(self.ctx.info().ctx_info()))
        } else {
            None
        };
        let upload = self
            .ctx
            .media()
            .upload(self.bytes, MediaType::Image)
            .await?;
        let message = whatsapp::Message {
            sticker_message: Some(Box::new(StickerMessage {
                url: Some(upload.url.clone()),
                file_sha256: Some(upload.file_sha256.to_vec()),
                file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                media_key: Some(upload.media_key.to_vec()),
                media_key_timestamp: Some(upload.media_key_timestamp),
                mimetype: Some("image/webp".to_string()),
                direct_path: Some(upload.direct_path.clone()),
                file_length: Some(upload.file_length),
                context_info: quoted,
                png_thumbnail: self.thumbnail,
                ..Default::default()
            })),
            ..Default::default()
        };
        self.ctx.message().raw(message).await
    }
}

impl<'a> IntoFuture for StickerBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
