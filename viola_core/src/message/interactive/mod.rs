pub mod cta_url;
pub mod inapp_signup;
pub mod quick_reply;
pub mod single_select;

use std::pin::Pin;

use whatsapp_rust::{
    anyhow,
    buffa::MessageField,
    waproto::whatsapp::{
        self,
        message::{
            InteractiveMessage,
            interactive_message::{self, Body, Footer, Header},
        },
    },
};

use crate::{
    context::Context,
    message::interactive::{
        cta_url::{CtaButton, CtaUrlBuilder},
        inapp_signup::InappSignupBuilder,
        quick_reply::{QuickReplyBuilder, QuickReplyButton},
        single_select::{SingleSelectBuilder, SingleSelectSection},
    },
};

pub struct InteractiveFactory<'a> {
    pub ctx: &'a Context,
}

impl<'a> InteractiveFactory<'a> {
    pub fn raw(
        self,
        interactive: interactive_message::InteractiveMessage,
    ) -> InteractiveBuilder<'a> {
        InteractiveBuilder {
            body: MessageField::none(),
            ctx: self.ctx,
            header: MessageField::none(),
            footer: MessageField::none(),
            interactive: Some(interactive),
            quoted: false,
        }
    }
    pub fn inapp_signup(self, text_body: impl Into<String>) -> InappSignupBuilder<'a> {
        InappSignupBuilder {
            ctx: self.ctx,
            quoted: false,
            text_body: Some(text_body.into()),
            title: None,
        }
    }

    pub fn quick_reply(self, buttons: Vec<QuickReplyButton>) -> QuickReplyBuilder<'a> {
        QuickReplyBuilder {
            quoted: false,
            ctx: self.ctx,
            footer: None,
            text_body: None,
            title: None,
            buttons,
        }
    }

    pub fn single_select(self, sections: Vec<SingleSelectSection>) -> SingleSelectBuilder<'a> {
        SingleSelectBuilder {
            ctx: self.ctx,
            quoted: false,
            title: None,
            footer: None,
            text_body: None,
            select_label: None,
            sections,
        }
    }

    pub fn cta_url(self, cta: Vec<CtaButton>) -> CtaUrlBuilder<'a> {
        CtaUrlBuilder {
            ctx: self.ctx,
            quoted: false,
            title: None,
            text_body: None,
            footer: None,
            cta,
        }
    }
}

pub struct InteractiveBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub body: MessageField<Body>,
    pub header: MessageField<Header>,
    pub footer: MessageField<Footer>,
    pub interactive: Option<interactive_message::InteractiveMessage>,
}

impl<'a> InteractiveBuilder<'a> {
    pub fn body(mut self, body: Body) -> Self {
        self.body = MessageField::some(body);
        self
    }

    pub fn header(mut self, header: Header) -> Self {
        self.header = MessageField::some(header);
        self
    }

    pub fn footer(mut self, footer: Footer) -> Self {
        self.footer = MessageField::some(footer);
        self
    }

    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let quoted = if self.quoted {
            MessageField::some(self.ctx.build_ctx_info())
        } else {
            MessageField::none()
        };

        self.ctx
            .send()
            .raw(whatsapp::Message {
                interactive_message: MessageField::some(InteractiveMessage {
                    header: self.header,
                    body: self.body,
                    interactive_message: self.interactive,
                    context_info: quoted,
                    footer: self.footer,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await
    }
}

impl<'a> IntoFuture for InteractiveBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
