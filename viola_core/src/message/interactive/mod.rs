pub mod carousel;
pub mod cta_url;
pub mod inapp_signup;
pub mod media;
pub mod quick_reply;
pub mod single_select;

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
    Context,
    message::{context_info_slot, sendable_builder},
};

pub struct InteractiveBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub header: MessageField<Header>,
    pub body: MessageField<Body>,
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

    pub async fn into_message(self) -> anyhow::Result<whatsapp::Message> {
        Ok(whatsapp::Message {
            interactive_message: MessageField::some(InteractiveMessage {
                header: self.header,
                body: self.body,
                footer: self.footer,
                interactive_message: self.interactive,
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

sendable_builder!(InteractiveBuilder);
