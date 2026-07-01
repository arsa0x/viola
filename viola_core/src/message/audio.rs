use crate::Context;
use std::pin::Pin;
use whatsapp_rust::{download::MediaType, media::AudioOptions};

pub struct AudioBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub quoted: bool,
    pub ptt: bool,
    pub mime_type: Option<String>,
    pub duration: Option<u32>,
}

impl<'a> AudioBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn ptt(mut self) -> Self {
        self.ptt = true;
        self
    }

    pub fn duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
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
            .upload(self.bytes, MediaType::Audio)
            .await?;

        let message = whatsapp_rust::media::audio_message(
            upload,
            AudioOptions {
                // mimetype: (),
                // duration_seconds: (),
                // ptt: (),
                // waveform: (),
                context_info: quoted,
                ..Default::default()
            },
        );

        self.ctx.send().raw(message).await
    }
}

impl<'a> IntoFuture for AudioBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
