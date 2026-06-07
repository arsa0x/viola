use crate::state::AppState;
use anyhow::anyhow;
use std::{sync::Arc, time::Instant};
use wacore::{download::MediaType, proto_helpers::MessageExt};
use waproto::whatsapp::{
    self, MessageKey,
    message::{AudioMessage, StickerMessage, VideoMessage},
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

    pub async fn get_media(&self) -> anyhow::Result<Vec<u8>> {
        let msg = &self.msg.message;

        let (url, media_key, _file_sha256, _media_type) = if let Some(img) = &msg.image_message {
            (
                img.url.as_deref(),
                img.media_key.as_ref(),
                img.file_sha256.as_ref(),
                MediaType::Image,
            )
        } else if let Some(vid) = &msg.video_message {
            (
                vid.url.as_deref(),
                vid.media_key.as_ref(),
                vid.file_sha256.as_ref(),
                MediaType::Video,
            )
        } else if let Some(aud) = &msg.audio_message {
            (
                aud.url.as_deref(),
                aud.media_key.as_ref(),
                aud.file_sha256.as_ref(),
                MediaType::Audio,
            )
        } else if let Some(doc) = &msg.document_message {
            (
                doc.url.as_deref(),
                doc.media_key.as_ref(),
                doc.file_sha256.as_ref(),
                MediaType::Document,
            )
        } else if let Some(stk) = &msg.sticker_message {
            (
                stk.url.as_deref(),
                stk.media_key.as_ref(),
                stk.file_sha256.as_ref(),
                MediaType::Sticker,
            )
        } else {
            return Err(anyhow!("No downloadable media found in this message"));
        };

        let _url = url.ok_or_else(|| anyhow!("Media URL is missing"))?;
        let _media_key = media_key.ok_or_else(|| anyhow!("Media key is missing"))?;

        // let bytes = self.client.download(url, media_key, media_type).await?;

        // Ok(bytes)
        Ok(Vec::new())
    }

    pub async fn get_media_url(&self) -> anyhow::Result<String> {
        let msg = &self.msg.message;

        let url = if let Some(img) = &msg.image_message {
            img.url.as_deref()
        } else if let Some(vid) = &msg.video_message {
            vid.url.as_deref()
        } else if let Some(aud) = &msg.audio_message {
            aud.url.as_deref()
        } else if let Some(doc) = &msg.document_message {
            doc.url.as_deref()
        } else {
            None
        };

        url.map(|u| u.to_string())
            .ok_or_else(|| anyhow!("No media URL found"))
    }

    fn generate_jpeg_thumbnail(&self, _: &[u8]) -> Option<Vec<u8>> {
        // to do
        None
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
        let msg = &self.msg.message;

        if let Some(text) = msg.text_content() {
            return Some(text);
        }

        if let Some(image) = &msg.image_message {
            if let Some(caption) = &image.caption {
                return Some(caption);
            }
        }

        if let Some(video) = &msg.video_message {
            if let Some(caption) = &video.caption {
                return Some(caption);
            }
        }

        if let Some(document) = &msg.document_message {
            if let Some(caption) = &document.caption {
                return Some(caption);
            }
        }

        None
    }
}
