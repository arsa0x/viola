pub mod interactive;
pub mod media;
pub mod reaction;
pub mod text;

use whatsapp_rust::{
    anyhow,
    buffa::MessageField,
    waproto::whatsapp::{self, ContextInfo},
};

use crate::{
    Context,
    message::{
        interactive::{
            InteractiveBuilder,
            carousel::CarouselBuilder,
            cta_url::{CtaButton, CtaUrlBuilder},
            inapp_signup::InappSignupBuilder,
            quick_reply::{QuickReplyBuilder, QuickReplyButton},
            single_select::{SingleSelectBuilder, SingleSelectSection},
        },
        media::{
            MediaSource, audio::AudioBuilder, document::DocumentBuilder, image::ImageBuilder,
            sticker::StickerBuilder, video::VideoBuilder,
        },
        reaction::ReactionBuilder,
        text::TextBuilder,
    },
};

pub struct MessageFactory<'a> {
    pub ctx: &'a Context,
}

impl<'a> MessageFactory<'a> {
    pub async fn raw(&self, message: whatsapp::Message) -> anyhow::Result<()> {
        self.ctx
            .wa_client
            .send_message(self.ctx.info.source.chat.clone(), message)
            .await?;
        Ok(())
    }

    pub fn text(&self, text: impl Into<String>) -> TextBuilder<'a> {
        TextBuilder::new(self.ctx, text)
    }

    pub fn image(&self, source: MediaSource<'a>) -> ImageBuilder<'a> {
        ImageBuilder {
            ctx: self.ctx,
            source,
            caption: None,
            thumbnail: None,
            quoted: false,
        }
    }

    pub fn video(&self, source: MediaSource<'a>) -> VideoBuilder<'a> {
        VideoBuilder {
            ctx: self.ctx,
            source,
            caption: None,
            thumbnail: None,
            quoted: false,
        }
    }

    pub fn audio(&self, source: MediaSource<'a>) -> AudioBuilder<'a> {
        AudioBuilder {
            ctx: self.ctx,
            source,
            quoted: false,
            ptt: None,
        }
    }

    pub fn document(&self, source: MediaSource<'a>) -> DocumentBuilder<'a> {
        DocumentBuilder {
            ctx: self.ctx,
            source,
            thumbnail: None,
            caption: None,
            quoted: false,
        }
    }

    pub fn sticker(&self, source: MediaSource<'a>) -> StickerBuilder<'a> {
        StickerBuilder {
            ctx: self.ctx,
            source,
            thumbnail: None,
            quoted: false,
        }
    }

    pub fn reaction(&self, react: impl Into<String>) -> ReactionBuilder<'a> {
        ReactionBuilder {
            ctx: self.ctx,
            reaction: Some(react.into()),
        }
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

    pub fn raw_interactive(&self) -> InteractiveBuilder<'a> {
        InteractiveBuilder {
            ctx: self.ctx,
            quoted: false,
            header: MessageField::none(),
            body: MessageField::none(),
            footer: MessageField::none(),
            interactive: None,
        }
    }

    pub fn cta_url(&self, cta: Vec<CtaButton>) -> CtaUrlBuilder<'a> {
        CtaUrlBuilder::new(self.ctx, cta)
    }

    pub fn inapp_signup(&self, text: impl Into<String>) -> InappSignupBuilder<'a> {
        InappSignupBuilder::new(self.ctx, text.into())
    }

    pub fn quick_reply(&self, buttons: Vec<QuickReplyButton>) -> QuickReplyBuilder<'a> {
        QuickReplyBuilder::new(self.ctx, buttons)
    }

    pub fn single_select(&self, sections: Vec<SingleSelectSection>) -> SingleSelectBuilder<'a> {
        SingleSelectBuilder::new(self.ctx, sections)
    }

    pub fn carousel(&self, text: impl Into<String>) -> CarouselBuilder<'a> {
        CarouselBuilder::new(self.ctx, text.into())
    }
}

pub trait ContextInfoSlot {
    fn absent() -> Self;
    fn present(info: ContextInfo) -> Self;
}

impl ContextInfoSlot for MessageField<ContextInfo> {
    fn absent() -> Self {
        MessageField::none()
    }
    fn present(info: ContextInfo) -> Self {
        MessageField::some(info)
    }
}

impl ContextInfoSlot for Option<Box<ContextInfo>> {
    fn absent() -> Self {
        None
    }
    fn present(info: ContextInfo) -> Self {
        Some(Box::new(info))
    }
}

pub fn context_info_slot<T: ContextInfoSlot>(ctx: &Context, quoted: bool) -> T {
    if quoted {
        T::present(ctx.build_ctx_info())
    } else {
        T::absent()
    }
}

macro_rules! sendable_builder {
    ($builder:ident) => {
        impl<'a> $builder<'a> {
            pub async fn send(self) -> anyhow::Result<()> {
                let ctx = self.ctx;
                let message = self.into_message().await?;
                ctx.send().raw(message).await
            }
        }

        impl<'a> IntoFuture for $builder<'a> {
            type Output = anyhow::Result<()>;
            type IntoFuture =
                std::pin::Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;
            fn into_future(self) -> Self::IntoFuture {
                Box::pin(self.send())
            }
        }
    };
}

pub(crate) use sendable_builder;
