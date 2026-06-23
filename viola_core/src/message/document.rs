use crate::context::Context;
use whatsapp_rust::{
    download::MediaType,
    waproto::whatsapp::{self, message::DocumentMessage},
};

pub struct DocumentBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub caption: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
}

impl<'a> DocumentBuilder<'a> {
    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

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
            document_message: Some(Box::new(DocumentMessage {
                url: Some(upload.url.clone()),
                file_sha256: Some(upload.file_sha256.to_vec()),
                file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                media_key: Some(upload.media_key.to_vec()),
                media_key_timestamp: Some(upload.media_key_timestamp),
                mimetype: None, // to do
                direct_path: Some(upload.direct_path.clone()),
                file_length: Some(upload.file_length),
                context_info: quoted,
                jpeg_thumbnail: self.thumbnail,
                caption: self.caption,
                ..Default::default()
            })),
            ..Default::default()
        };
        self.ctx.message().raw(message).await
    }
}
