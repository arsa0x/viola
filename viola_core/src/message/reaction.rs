use whatsapp_rust::{
    anyhow,
    buffa::MessageField,
    waproto::whatsapp::{self, MessageKey},
};

use crate::{Context, message::sendable_builder};

pub struct ReactionBuilder<'a> {
    pub ctx: &'a Context,
    pub reaction: Option<String>,
}

impl<'a> ReactionBuilder<'a> {
    pub async fn into_message(self) -> anyhow::Result<whatsapp::Message> {
        let chat = self.ctx.info.source.chat.clone();
        let sender = self.ctx.info.source.sender.clone();
        let id = self.ctx.info.id.clone();

        let key = MessageKey {
            from_me: Some(false),
            remote_jid: Some(chat.to_string()),
            id: Some(id),
            participant: Some(sender.to_string()),
        };

        Ok(whatsapp::Message {
            reaction_message: MessageField::some(whatsapp::message::ReactionMessage {
                key: MessageField::some(key),
                text: self.reaction,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

sendable_builder!(ReactionBuilder);
