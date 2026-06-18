use crate::context::Context;
use whatsapp_rust::{
    download::MediaType,
    waproto::whatsapp::{self, message::StickerMessage},
};

pub struct StickerBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
}

impl<'a> StickerBuilder<'a> {
    pub fn thumbnail(mut self, thumbnail: Vec<u8>) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }
}

impl<'a> IntoFuture for StickerBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let upload = self
                .ctx
                .media()
                .upload(self.bytes, MediaType::Image)
                .await?;
            let reply = whatsapp::Message {
                sticker_message: Some(Box::new(StickerMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256.to_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                    media_key: Some(upload.media_key.to_vec()),
                    media_key_timestamp: Some(upload.media_key_timestamp),
                    mimetype: Some("image/webp".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(upload.file_length),
                    context_info: Some(Box::new(self.ctx.info().ctx_info())),
                    png_thumbnail: self.thumbnail,
                    ..Default::default()
                })),
                ..Default::default()
            };
            self.ctx.send().message(reply).await
        })
    }
}
