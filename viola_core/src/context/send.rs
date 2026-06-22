use crate::{
    context::Context,
    media::{
        audio::AudioBuilder, document::DocumentBuilder, image::ImageBuilder,
        sticker::StickerBuilder, video::VideoBuilder,
    },
};
use anyhow::Result;
use whatsapp_rust::{
    proto_helpers::MessageBuilderExt,
    waproto::whatsapp::{self, MessageKey, message::ExtendedTextMessage},
};

pub struct Sender<'a> {
    pub ctx: &'a Context,
}

impl<'a> Sender<'a> {
    pub async fn text(&self, text: &str) -> Result<()> {
        let reply = whatsapp::Message::text(text);
        self.ctx.send().message(reply).await
    }

    pub async fn quoted_text(&self, text: &str) -> Result<()> {
        let reply = whatsapp::Message {
            extended_text_message: Some(Box::new(ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(self.ctx.info().ctx_info())),
                ..Default::default()
            })),
            ..Default::default()
        };
        self.ctx.send().message(reply).await
    }

    pub async fn message(&self, message: whatsapp::Message) -> Result<()> {
        self.ctx
            .msg_ctx
            .client
            .send_message(self.ctx.info().chat_jid(), message)
            .await?;
        Ok(())
    }

    pub fn video(&self, video: Vec<u8>) -> VideoBuilder<'a> {
        VideoBuilder {
            ctx: self.ctx,
            bytes: video,
            caption: None,
            thumbnail: None,
        }
    }

    pub fn audio(&self, audio: Vec<u8>) -> AudioBuilder<'a> {
        AudioBuilder {
            ctx: self.ctx,
            bytes: audio,
        }
    }

    pub fn image(&self, image: Vec<u8>) -> ImageBuilder<'a> {
        ImageBuilder {
            bytes: image,
            ctx: self.ctx,
            caption: None,
            thumbnail: None,
            quoted: false,
        }
    }

    pub fn document(&self, document: Vec<u8>) -> DocumentBuilder<'a> {
        DocumentBuilder {
            ctx: self.ctx,
            bytes: document,
            caption: None,
            thumbnail: None,
        }
    }

    pub fn sticker(&self, sticker: Vec<u8>) -> StickerBuilder<'a> {
        StickerBuilder {
            ctx: self.ctx,
            bytes: sticker,
            thumbnail: None,
        }
    }

    pub async fn react(&self, emoji: &str) -> Result<()> {
        let key = MessageKey {
            remote_jid: Some(self.ctx.info().chat_jid().to_string()),
            participant: Some(self.ctx.info().sender_jid().to_string()),
            id: Some(self.ctx.msg_ctx.info.id.to_string()),
            from_me: Some(false),
        };

        self.ctx
            .msg_ctx
            .client
            .send_reaction(&self.ctx.info().chat_jid(), key, emoji)
            .await?;

        Ok(())
    }

    pub async fn reply_wait(&self) -> Result<()> {
        self.react("⏳").await
    }

    pub async fn reply_success(&self) -> Result<()> {
        self.react("✅").await
    }

    pub async fn reply_failed(&self) -> Result<()> {
        self.react("❌").await
    }

    pub async fn reply_error(&self, err: impl std::fmt::Display) -> Result<()> {
        self.reply_failed().await?;
        self.quoted_text(&format!("Error:\n{}", err)).await
    }
}
