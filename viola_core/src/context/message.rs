use crate::context::{Context, media::Thumbnail};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use whatsapp_rust::{
    download::{Downloadable, MediaType},
    upload::UploadResponse,
    waproto::whatsapp::{
        self,
        message::{AudioMessage, ImageMessage, StickerMessage, VideoMessage},
    },
};

pub enum MediaSource {
    Url(String),
    Bytes(Bytes),
}

pub struct MessageBuilder<'a> {
    pub ctx: &'a Context,
    pub upload: UploadResponse,
    pub length: u64,
}

impl Context {
    pub fn get_media<'a>(&'a self) -> anyhow::Result<(&'a dyn Downloadable, MediaType)> {
        let msg = &self.msg_ctx.message;

        Self::extract_media_from_proto(msg)
    }

    pub async fn media_message(
        &self,
        data: Vec<u8>,
        media_type: MediaType,
        caption: Option<String>,
    ) -> Result<whatsapp::Message> {
        let length = data.len() as u64;
        let thumb = match &media_type {
            MediaType::Image => Thumbnail::image_thumbnail_from_memory(&data),
            MediaType::Video => Thumbnail::video_thumbnail(&data),
            _ => None,
        };
        let upload = self
            .msg_ctx
            .client
            .upload(data, media_type, Default::default())
            .await?;

        let msg = MessageBuilder {
            ctx: self,
            length,
            upload,
        };

        match &media_type {
            MediaType::Audio => Ok(msg.audio_message()),
            MediaType::Image => Ok(msg.image_message(thumb, caption)),
            MediaType::Video => Ok(msg.video_message(thumb, caption)),
            MediaType::Sticker => Ok(msg.sticker_message(thumb)),
            _ => Err(anyhow!("unsupport media message")),
        }
    }

    fn extract_media_from_proto(
        msg: &whatsapp::Message,
    ) -> anyhow::Result<(&dyn Downloadable, MediaType)> {
        if let Some(ext) = &msg.extended_text_message {
            if let Some(context_info) = &ext.context_info {
                if let Some(quoted_msg) = &context_info.quoted_message {
                    return Self::extract_media_from_proto(quoted_msg);
                }
            }
            Err(anyhow::anyhow!(
                "extended text message does not contain a quoted media message"
            ))
        } else if let Some(onc) = &msg.view_once_message {
            if let Some(once_msg) = &onc.message {
                return Self::extract_media_from_proto(once_msg);
            }
            Err(anyhow::anyhow!("cannot get media"))
        } else if let Some(onc_v2) = &msg.view_once_message_v2 {
            if let Some(once_msg) = &onc_v2.message {
                return Self::extract_media_from_proto(once_msg);
            }
            Err(anyhow::anyhow!("cannot get media"))
        } else {
            if let Some(img) = &msg.image_message {
                Ok((img.as_ref() as &dyn Downloadable, MediaType::Image))
            } else if let Some(vid) = &msg.video_message {
                Ok((vid.as_ref() as &dyn Downloadable, MediaType::Video))
            } else if let Some(aud) = &msg.audio_message {
                Ok((aud.as_ref() as &dyn Downloadable, MediaType::Audio))
            } else if let Some(doc) = &msg.document_message {
                Ok((doc.as_ref() as &dyn Downloadable, MediaType::Document))
            } else if let Some(stk) = &msg.sticker_message {
                Ok((stk.as_ref() as &dyn Downloadable, MediaType::Sticker))
            } else {
                Err(anyhow::anyhow!(
                    "no downloadable media found in this message"
                ))
            }
        }
    }
}

impl<'a> MessageBuilder<'a> {
    pub fn video_message(
        &self,
        thumbnail_bytes: Option<Vec<u8>>,
        caption: Option<String>,
    ) -> whatsapp::Message {
        whatsapp::Message {
            video_message: Some(Box::new(VideoMessage {
                url: Some(self.upload.url.clone()),
                file_sha256: Some(self.upload.file_sha256.to_vec()),
                file_enc_sha256: Some(self.upload.file_enc_sha256.to_vec()),
                media_key: Some(self.upload.media_key.to_vec()),
                media_key_timestamp: Some(self.upload.media_key_timestamp),
                mimetype: Some("video/mp4".to_string()),
                direct_path: Some(self.upload.direct_path.clone()),
                file_length: Some(self.length),
                context_info: Some(Box::new(self.ctx.ctx_info())),
                jpeg_thumbnail: thumbnail_bytes,
                caption,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    pub fn image_message(
        &self,
        thumbnail_bytes: Option<Vec<u8>>,
        caption: Option<String>,
    ) -> whatsapp::Message {
        whatsapp::Message {
            image_message: Some(Box::new(ImageMessage {
                url: Some(self.upload.url.clone()),
                file_sha256: Some(self.upload.file_sha256.to_vec()),
                file_enc_sha256: Some(self.upload.file_enc_sha256.to_vec()),
                media_key: Some(self.upload.media_key.to_vec()),
                media_key_timestamp: Some(self.upload.media_key_timestamp),
                mimetype: Some("image/jpeg".to_string()),
                direct_path: Some(self.upload.direct_path.clone()),
                file_length: Some(self.length),
                context_info: Some(Box::new(self.ctx.ctx_info())),
                jpeg_thumbnail: thumbnail_bytes,
                caption,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    pub fn audio_message(&self) -> whatsapp::Message {
        whatsapp::Message {
            audio_message: Some(Box::new(AudioMessage {
                url: Some(self.upload.url.clone()),
                file_sha256: Some(self.upload.file_sha256.to_vec()),
                file_enc_sha256: Some(self.upload.file_enc_sha256.to_vec()),
                media_key: Some(self.upload.media_key.to_vec()),
                media_key_timestamp: Some(self.upload.media_key_timestamp),
                mimetype: Some("audio/mpeg".to_string()),
                direct_path: Some(self.upload.direct_path.clone()),
                file_length: Some(self.length),
                context_info: Some(Box::new(self.ctx.ctx_info())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    pub fn sticker_message(&self, thumbnail_bytes: Option<Vec<u8>>) -> whatsapp::Message {
        whatsapp::Message {
            sticker_message: Some(Box::new(StickerMessage {
                url: Some(self.upload.url.clone()),
                file_sha256: Some(self.upload.file_sha256.to_vec()),
                file_enc_sha256: Some(self.upload.file_enc_sha256.to_vec()),
                media_key: Some(self.upload.media_key.to_vec()),
                media_key_timestamp: Some(self.upload.media_key_timestamp),
                mimetype: Some("image/webp".to_string()),
                direct_path: Some(self.upload.direct_path.clone()),
                file_length: Some(self.length),
                png_thumbnail: thumbnail_bytes,
                context_info: Some(Box::new(self.ctx.ctx_info())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}
