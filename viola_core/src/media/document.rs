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
}

impl<'a> IntoFuture for DocumentBuilder<'a> {
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
                document_message: Some(Box::new(DocumentMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256.to_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                    media_key: Some(upload.media_key.to_vec()),
                    media_key_timestamp: Some(upload.media_key_timestamp),
                    mimetype: None, // to do
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(upload.file_length),
                    context_info: Some(Box::new(self.ctx.info().ctx_info())),
                    jpeg_thumbnail: self.thumbnail,
                    caption: self.caption,
                    ..Default::default()
                })),
                ..Default::default()
            };
            self.ctx.send().message(reply).await
        })
    }
}
