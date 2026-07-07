use crate::context::Context;
use anyhow::{Result, anyhow};
use whatsapp_rust::{
    download::{Downloadable, MediaType},
    upload::UploadResponse,
    waproto::whatsapp,
};

pub enum MediaRef<'a> {
    Image(&'a dyn Downloadable),
    Video(&'a dyn Downloadable),
    Audio(&'a dyn Downloadable),
    Sticker(&'a dyn Downloadable),
    Document(&'a dyn Downloadable),
}

pub struct Media<'a> {
    pub ctx: &'a Context,
}

impl<'a> Media<'a> {
    pub async fn download(&self, dl: &'a dyn Downloadable) -> Result<Vec<u8>> {
        Ok(self.ctx.msg_ctx.client.download(dl).await?)
    }

    pub async fn upload(&self, media: Vec<u8>, media_type: MediaType) -> Result<UploadResponse> {
        self.ctx
            .msg_ctx
            .client
            .upload(media, media_type, Default::default())
            .await
    }

    pub fn current(&self) -> Result<MediaRef<'_>> {
        Self::extract_media(&self.ctx.msg_ctx.message)
    }

    pub fn quoted(&self) -> Result<MediaRef<'_>> {
        let msg = &self.ctx.msg_ctx.message;

        let ext = msg
            .extended_text_message
            .as_option()
            .ok_or_else(|| anyhow::anyhow!("not a reply message"))?;

        let quoted = ext
            .context_info
            .as_option()
            .and_then(|ctx| ctx.quoted_message.as_option())
            .ok_or_else(|| anyhow::anyhow!("quoted message not found"))?;

        Self::extract_media(quoted)
    }

    fn extract_media(msg: &'a whatsapp::Message) -> Result<MediaRef<'a>> {
        if let Some(vo) = msg.view_once_message.as_option() {
            if let Some(inner) = vo.message.as_option() {
                return Self::extract_media(inner);
            }
        }

        if let Some(vo) = msg.view_once_message_v2.as_option() {
            if let Some(inner) = vo.message.as_option() {
                return Self::extract_media(inner);
            }
        }

        if let Some(img) = msg.image_message.as_option() {
            return Ok(MediaRef::Image(img));
        }

        if let Some(video) = msg.video_message.as_option() {
            return Ok(MediaRef::Video(video));
        }

        if let Some(audio) = msg.audio_message.as_option() {
            return Ok(MediaRef::Audio(audio));
        }

        if let Some(sticker) = msg.sticker_message.as_option() {
            return Ok(MediaRef::Sticker(sticker));
        }

        if let Some(doc) = msg.document_message.as_option() {
            return Ok(MediaRef::Document(doc));
        }

        Err(anyhow!("quoted message does not contain media"))
    }
}
