use crate::context::Context;
use whatsapp_rust::{anyhow, download::MediaType, media::DocumentOptions};

pub struct DocumentBuilder<'a> {
    pub ctx: &'a Context,
    pub bytes: Vec<u8>,
    pub caption: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub title: Option<String>,
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

    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let quoted = if self.quoted {
            Some(Box::new(self.ctx.build_ctx_info()))
        } else {
            None
        };

        let upload = self
            .ctx
            .wa_client
            .upload(self.bytes, MediaType::Image, Default::default())
            .await?;

        let message = whatsapp_rust::media::document_message(
            upload,
            DocumentOptions {
                // mimetype: (),
                // file_name: (),
                // title: (),
                // page_count: (),
                context_info: quoted,
                jpeg_thumbnail: self.thumbnail,
                caption: self.caption,
                ..Default::default()
            },
        );

        self.ctx.send().raw(message).await
    }
}
