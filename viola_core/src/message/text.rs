use whatsapp_rust::{anyhow, buffa::MessageField, waproto::whatsapp};

use crate::{
    Context,
    message::{context_info_slot, sendable_builder},
};

pub struct TextBuilder<'a> {
    pub ctx: &'a Context,
    pub message: whatsapp::Message,
    pub quoted: bool,
}

impl<'a> TextBuilder<'a> {
    pub fn new(ctx: &'a Context, text: impl Into<String>) -> Self {
        Self {
            ctx,
            message: whatsapp::Message {
                conversation: Some(text.into()),
                ..Default::default()
            },
            quoted: false,
        }
    }

    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub async fn into_message(mut self) -> anyhow::Result<whatsapp::Message> {
        if self.quoted {
            if let Some(text) = self.message.conversation.take() {
                self.message.extended_text_message =
                    MessageField::some(whatsapp::message::ExtendedTextMessage {
                        text: Some(text),
                        context_info: context_info_slot(self.ctx, true),
                        ..Default::default()
                    })
            }
        }
        Ok(self.message)
    }
}

sendable_builder!(TextBuilder);
