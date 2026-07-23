use whatsapp_rust::{anyhow, waproto::whatsapp};

use crate::{
    Context,
    message::{context_info_slot, media::MediaSource, sendable_builder},
};

pub struct DocumentBuilder<'a> {
    pub ctx: &'a Context,
    pub source: MediaSource<'a>,
    pub thumbnail: Option<Vec<u8>>,
    pub caption: Option<String>,
    pub quoted: bool,
}

impl<'a> DocumentBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

    pub fn thumbnail(mut self, thumbnail: Vec<u8>) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    pub async fn into_message(self) -> anyhow::Result<whatsapp::Message> {
        let bytes = self.source.get_media_bytes(self.ctx).await?;
        let upload = self
            .ctx
            .wa_client
            .upload(
                bytes,
                whatsapp_rust::download::MediaType::Document,
                Default::default(),
            )
            .await?;
        Ok(whatsapp_rust::media::document_message(
            upload,
            whatsapp_rust::media::DocumentOptions {
                // mimetype: (),
                // file_name: (),
                // title: (),
                caption: self.caption,
                jpeg_thumbnail: self.thumbnail,
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            },
        ))
    }
}

sendable_builder!(DocumentBuilder);
