use crate::state::AppState;
use anyhow::anyhow;
use futures::AsyncReadExt;
use isahc::get_async;
use std::{sync::Arc, time::Instant};
use wacore::{download::MediaType, proto_helpers::MessageExt, types::message::MessageInfo};
use waproto::whatsapp::{
    self,
    message::{AudioMessage, StickerMessage, VideoMessage},
};
use whatsapp_rust::{Client, bot::MessageContext};

#[derive(Clone)]
pub struct Context {
    pub msg: MessageContext,
    pub text: String,
    pub command: String,
    pub args: Vec<String>,
    pub state: Arc<AppState>,
    pub created_at: Instant,
}

pub enum MediaSource {
    Url(String),
    Bytes(Vec<u8>),
}

impl Context {
    pub fn new(
        message: &Arc<waproto::whatsapp::Message>,
        info: &Arc<MessageInfo>,
        client: Arc<Client>,
        state: Arc<AppState>,
    ) -> Self {
        Self {
            msg: MessageContext::from_parts(message, info, client),
            text: String::new(),
            command: String::new(),
            args: Vec::new(),
            state,
            created_at: Instant::now(),
        }
    }
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

        self.msg.send_message(reply).await?;
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
                let mut response = get_async(url).await?;
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes).await?;
                bytes
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
    pub async fn get_media(&self) {} // to do
    pub async fn get_media_url(&self) {} // to do

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

    pub fn elapsed_ms(&self) -> u128 {
        self.created_at.elapsed().as_millis()
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

    fn split_arguments(&self, input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in input.chars() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' | '\n' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    pub fn parse_command(mut self, prefix: &str) -> Option<Self> {
        let text = self.text_content()?.to_owned();
        if !text.starts_with(prefix) {
            return None;
        }
        let without_prefix = text.trim_start_matches(prefix);
        let parts = self.split_arguments(without_prefix);

        let (cmd, args) = parts.split_first()?;

        self.text = text;
        self.command = cmd.to_lowercase();
        self.args = args.to_vec();

        // if let Some(text) = text {
        //     self.text = text.clone();

        //     if text.starts_with(prefix) {
        //         let without_prefix = text.trim_start_matches(prefix);

        //         let parts = self.split_arguments(without_prefix);

        //         if let Some((cmd, args)) = parts.split_first() {
        //             self.command = cmd.to_lowercase();
        //             self.args = args.to_vec();
        //         }
        //     }
        // }

        Some(self)
    }
}
