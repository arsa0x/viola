use crate::{Context, message::media::MediaSource};
use std::pin::Pin;
use whatsapp_rust::{anyhow, download::MediaType, media::AudioOptions};

pub struct AudioBuilder<'a> {
    pub ctx: &'a Context,
    pub source: MediaSource<'a>,
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
            Some(Box::new(self.ctx.build_ctx_info()))
        } else {
            None
        };

        let bytes = self.source.get_media(self.ctx).await?;

        let upload = self
            .ctx
            .wa_client
            .upload(bytes, MediaType::Audio, Default::default())
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
