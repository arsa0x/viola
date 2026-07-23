use crate::{Context, message::media::MediaSource};
use std::pin::Pin;
use whatsapp_rust::{anyhow, download::MediaType, media::VideoOptions};

pub struct VideoBuilder<'a> {
    pub ctx: &'a Context,
    pub source: MediaSource<'a>,
    pub caption: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
}

impl<'a> VideoBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn thumbnail(mut self, thumbnail: Vec<u8>) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let quoted = if self.quoted {
            Some(Box::new(self.ctx.build_ctx_info()))
        } else {
            None
        };

        let bytes = self.source.get_media(self.ctx).await?;

        let thumbnail = match self.thumbnail {
            Some(t) => Some(t),
            None => Some(Vec::new()), // to do
        };

        let upload = self
            .ctx
            .wa_client
            .upload(bytes, MediaType::Video, Default::default())
            .await?;

        let message = whatsapp_rust::media::video_message(
            upload,
            VideoOptions {
                context_info: quoted,
                jpeg_thumbnail: thumbnail,
                caption: self.caption,
                ..Default::default()
            },
        );
        self.ctx.send().raw(message).await
    }
}

impl<'a> IntoFuture for VideoBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
