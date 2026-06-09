use crate::{state::AppState, utils};
use anyhow::anyhow;
use std::{io::Cursor, sync::Arc, time::Instant};
use wacore::download::{Downloadable, MediaType};
use waproto::whatsapp::{
    self, Message, MessageKey,
    message::{AudioMessage, ImageMessage, StickerMessage, VideoMessage},
};
use whatsapp_rust::{Client, bot::MessageContext};

#[derive(Clone)]
pub struct Context {
    pub msg: MessageContext,
    pub client: Arc<Client>,
    pub args: Vec<String>,
    pub state: Arc<AppState>,
    pub created_at: Instant,
}

pub enum MediaSource {
    Url(String),
    Bytes(Vec<u8>),
}

impl Context {
    pub async fn reply(&self, text: &str) -> anyhow::Result<()> {
        let ctx_info = self.msg.build_quote_context();
        let reply = whatsapp::Message {
            extended_text_message: Some(Box::new(whatsapp::message::ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(ctx_info)),
                ..Default::default()
            })),
            ..Default::default()
        };

        let jid = self.msg.info.source.chat.clone();
        self.client.send_message(jid, reply).await?;

        Ok(())
    }

    pub fn processing_ms(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64() * 1000.0
    }

    pub async fn send_reaction(&self, emoji: &str) -> anyhow::Result<()> {
        let chat_jid = self.msg.info.source.chat.clone();
        let jid = self.msg.info.source.sender.clone();

        let target_key = MessageKey {
            remote_jid: Some(chat_jid.to_string()),
            from_me: Some(false),
            id: Some(self.msg.info.id.to_string()),
            participant: Some(jid.to_string()),
        };
        self.client
            .send_reaction(&chat_jid, target_key, emoji)
            .await?;
        Ok(())
    }

    pub async fn reply_wait(&self) -> anyhow::Result<()> {
        self.send_reaction("⏳").await?;
        Ok(())
    }

    pub async fn reply_success(&self) -> anyhow::Result<()> {
        self.send_reaction("✅️").await?;
        Ok(())
    }

    pub async fn reply_failed(&self) -> anyhow::Result<()> {
        self.send_reaction("❌").await?;

        Ok(())
    }

    pub async fn reply_voice_note(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn reply_media(
        &self,
        source: MediaSource,
        media_type: MediaType,
        caption: Option<String>,
    ) -> anyhow::Result<()> {
        let media_bytes = match source {
            MediaSource::Bytes(b) => b,
            MediaSource::Url(url) => {
                let response = self.state.http.get(url).send().await?;
                let bytes = response.bytes().await?;
                bytes.to_vec()
            }
        };

        let len = &media_bytes.len();

        let thumbnail_bytes = if media_type == MediaType::Image {
            self.generate_jpeg_thumbnail(&media_bytes)
        } else if media_type == MediaType::Video {
            let temp_path = self.state.dir.join("cache").join(&format!(
                "thumb_{}.mp4",
                std::time::Instant::now().elapsed().as_nanos()
            ));

            if std::fs::write(&temp_path, &media_bytes).is_ok() {
                let res = utils::generate_video_thumbnail(&temp_path.to_string_lossy()).ok();
                let _ = std::fs::remove_file(&temp_path);
                res
            } else {
                None
            }
        } else {
            None
        };

        let upload = self
            .msg
            .client
            .upload(media_bytes, media_type, Default::default())
            .await?;

        let ctx_info = self.msg.build_quote_context();
        let reply = match &media_type {
            MediaType::Audio => whatsapp::Message {
                audio_message: Some(Box::new(AudioMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                    media_key: Some(upload.media_key_vec()),
                    mimetype: Some("audio/mpeg".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(*len as u64),
                    context_info: Some(Box::new(ctx_info)),
                    ..Default::default()
                })),
                ..Default::default()
            },
            MediaType::Video => whatsapp::Message {
                video_message: Some(Box::new(VideoMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                    media_key: Some(upload.media_key_vec()),
                    mimetype: Some("video/mp4".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(*len as u64),
                    context_info: Some(Box::new(ctx_info)),
                    jpeg_thumbnail: thumbnail_bytes,
                    caption,
                    ..Default::default()
                })),
                ..Default::default()
            },
            MediaType::Image => whatsapp::Message {
                image_message: Some(Box::new(ImageMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                    media_key: Some(upload.media_key_vec()),
                    mimetype: Some("image/jpeg".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(*len as u64),
                    context_info: Some(Box::new(ctx_info)),
                    jpeg_thumbnail: thumbnail_bytes,
                    ..Default::default()
                })),
                ..Default::default()
            },
            MediaType::Sticker => whatsapp::Message {
                sticker_message: Some(Box::new(StickerMessage {
                    url: Some(upload.url.clone()),
                    file_sha256: Some(upload.file_sha256_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256_vec()),
                    media_key: Some(upload.media_key_vec()),
                    mimetype: Some("image/webp".to_string()),
                    direct_path: Some(upload.direct_path.clone()),
                    file_length: Some(*len as u64),
                    png_thumbnail: thumbnail_bytes,
                    context_info: Some(Box::new(ctx_info)),
                    ..Default::default()
                })),
                ..Default::default()
            },
            _ => whatsapp::Message {
                extended_text_message: Some(Box::new(whatsapp::message::ExtendedTextMessage {
                    text: caption,
                    context_info: Some(Box::new(ctx_info)),
                    ..Default::default()
                })),
                ..Default::default()
            },
        };

        self.msg.send_message(reply).await?;

        Ok(())
    }

    fn generate_jpeg_thumbnail(&self, media_bytes: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory(media_bytes).ok()?;
        let thumbnail = img.thumbnail(100, 100);

        let mut jpeg_bytes = Cursor::new(Vec::new());

        thumbnail
            .write_to(&mut jpeg_bytes, image::ImageFormat::Jpeg)
            .ok()?;
        Some(jpeg_bytes.into_inner())
    }

    pub fn get_media<'a>(&'a self) -> anyhow::Result<(&'a dyn Downloadable, MediaType)> {
        let msg = &self.msg.message;

        Self::extract_media_from_proto(msg)
    }

    fn extract_media_from_proto(msg: &Message) -> anyhow::Result<(&dyn Downloadable, MediaType)> {
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

    pub fn sender(&self) -> anyhow::Result<String> {
        let phone = self
            .msg
            .info
            .source
            .sender_alt
            .as_ref()
            .ok_or_else(|| anyhow!("Sender alternative info missing"))?
            .user_base();

        Ok(phone.to_string())
    }

    pub fn is_group(&self) -> bool {
        self.msg.info.source.is_group
    }

    pub fn text_content(&self) -> Option<&str> {
        utils::get_text_content(&self.msg.message)
    }
}
