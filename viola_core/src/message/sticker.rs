use crate::context::Context;
use std::pin::Pin;
use whatsapp_rust::{
    anyhow,
    buffa::MessageField,
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
            MessageField::some(self.ctx.build_ctx_info())
        } else {
            MessageField::none()
        };
        let thumbnail = match self.thumbnail {
            Some(t) => Some(t),
            None => Some(Vec::new()), // to do
        };
        let upload = self
            .ctx
            .wa_client
            .upload(self.bytes, MediaType::Image, Default::default())
            .await?;
        let message = whatsapp::Message {
            sticker_message: MessageField::some(StickerMessage {
                url: Some(upload.url.clone()),
                file_sha256: Some(upload.file_sha256.to_vec()),
                file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                media_key: Some(upload.media_key.to_vec()),
                media_key_timestamp: Some(upload.media_key_timestamp),
                mimetype: Some("image/webp".to_string()),
                direct_path: Some(upload.direct_path.clone()),
                file_length: Some(upload.file_length),
                context_info: quoted,
                png_thumbnail: thumbnail,
                ..Default::default()
            }),
            ..Default::default()
        };
        self.ctx.send().raw(message).await
    }
}

impl<'a> IntoFuture for StickerBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
