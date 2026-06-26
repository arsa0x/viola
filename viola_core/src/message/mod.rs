pub mod audio;
pub mod document;
pub mod extended;
pub mod image;
pub mod interactive;
pub mod reaction;
pub mod sticker;
pub mod text;
pub mod video;

use crate::{
    Context,
    message::{
        audio::AudioBuilder, document::DocumentBuilder, image::ImageBuilder,
        interactive::InteractiveFactory, reaction::ReactionBuilder, sticker::StickerBuilder,
        text::TextBuilder, video::VideoBuilder,
    },
};
use whatsapp_rust::waproto::whatsapp::{self};

pub struct MessageFactory<'a> {
    pub ctx: &'a Context,
}

pub struct BaseBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
}

impl<'a> MessageFactory<'a> {
    pub async fn raw(&self, message: whatsapp::Message) -> anyhow::Result<()> {
        self.ctx
            .msg_ctx
            .client
            .send_message(self.ctx.info().chat_jid(), message)
            .await?;
        Ok(())
    }

    pub fn text(self, text: impl Into<String>) -> TextBuilder<'a> {
        TextBuilder {
            ctx: self.ctx,
            quoted: false,
            text: text.into(),
        }
    }

    pub fn video(self, video: Vec<u8>) -> VideoBuilder<'a> {
        VideoBuilder {
            ctx: self.ctx,
            bytes: video,
            quoted: false,
            caption: None,
            thumbnail: None,
        }
    }

    pub fn image(self, image: Vec<u8>) -> ImageBuilder<'a> {
        ImageBuilder {
            ctx: self.ctx,
            bytes: image,
            quoted: false,
            caption: None,
            thumbnail: None,
        }
    }

    pub fn audio(self, audio: Vec<u8>) -> AudioBuilder<'a> {
        AudioBuilder {
            bytes: audio,
            ctx: self.ctx,
            quoted: false,
        }
    }

    pub fn document(self, document: Vec<u8>) -> DocumentBuilder<'a> {
        DocumentBuilder {
            bytes: document,
            caption: None,
            ctx: self.ctx,
            quoted: false,
            thumbnail: None,
        }
    }

    pub fn sticker(self, sticker: Vec<u8>) -> StickerBuilder<'a> {
        StickerBuilder {
            bytes: sticker,
            ctx: self.ctx,
            quoted: false,
            thumbnail: None,
        }
    }

    pub fn reaction(self, react: &'a str) -> ReactionBuilder<'a> {
        ReactionBuilder {
            ctx: self.ctx,
            reaction: react,
        }
    }

    pub fn interactive(self) -> InteractiveFactory<'a> {
        InteractiveFactory { ctx: self.ctx }
    }

    pub async fn success(self) -> anyhow::Result<()> {
        self.reaction("✅").await
    }

    pub async fn wait(self) -> anyhow::Result<()> {
        self.reaction("⏳").await
    }

    pub async fn failed(self) -> anyhow::Result<()> {
        self.reaction("❌").await
    }
}
