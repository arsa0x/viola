use crate::context::Context;
use whatsapp_rust::{
    download::MediaType,
    waproto::whatsapp::{self, message::AudioMessage},
};

pub struct AudioBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
}

impl<'a> IntoFuture for AudioBuilder<'a> {
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
                audio_message: Some(Box::new(AudioMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256.to_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                    media_key: Some(upload.media_key.to_vec()),
                    media_key_timestamp: Some(upload.media_key_timestamp),
                    mimetype: Some("audio/mpeg".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(upload.file_length),
                    context_info: Some(Box::new(self.ctx.info().ctx_info())),
                    ..Default::default()
                })),
                ..Default::default()
            };
            self.ctx.send().message(reply).await
        })
    }
}
