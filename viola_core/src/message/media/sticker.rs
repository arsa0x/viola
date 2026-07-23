use whatsapp_rust::{
    anyhow::{self, Ok},
    buffa::MessageField,
    waproto::whatsapp,
};

use crate::{
    Context,
    message::{context_info_slot, media::MediaSource, sendable_builder},
};

pub struct StickerBuilder<'a> {
    pub ctx: &'a Context,
    pub source: MediaSource<'a>,
    pub thumbnail: Option<Vec<u8>>,
    pub quoted: bool,
}

impl<'a> StickerBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
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
                whatsapp_rust::download::MediaType::Sticker,
                Default::default(),
            )
            .await?;

        Ok(whatsapp::Message {
            sticker_message: MessageField::some(whatsapp::message::StickerMessage {
                url: Some(upload.url.clone()),
                file_sha256: Some(upload.file_sha256.to_vec()),
                file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                media_key: Some(upload.media_key.to_vec()),
                media_key_timestamp: Some(upload.media_key_timestamp),
                mimetype: Some("image/webp".to_string()),
                direct_path: Some(upload.direct_path.clone()),
                file_length: Some(upload.file_length),
                context_info: context_info_slot(self.ctx, self.quoted),
                png_thumbnail: self.thumbnail,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

sendable_builder!(StickerBuilder);
