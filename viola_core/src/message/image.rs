use crate::Context;
use std::pin::Pin;
use whatsapp_rust::{anyhow, download::MediaType, media::ImageOptions};

pub struct ImageBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub caption: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
}

impl<'a> ImageBuilder<'a> {
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

        let thumbnail = match self.thumbnail {
            Some(t) => Some(t),
            None => Some(Vec::new()), // to do
        };

        let upload = self
            .ctx
            .wa_client
            .upload(self.bytes, MediaType::Image, Default::default())
            .await?;

        let message = whatsapp_rust::media::image_message(
            upload,
            ImageOptions {
                context_info: quoted,
                jpeg_thumbnail: thumbnail,
                caption: self.caption,
                ..Default::default()
            },
        );

        self.ctx.send().raw(message).await
    }
}

impl<'a> IntoFuture for ImageBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
