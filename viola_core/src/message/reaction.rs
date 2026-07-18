use crate::context::Context;
use std::pin::Pin;
use whatsapp_rust::{anyhow, waproto::whatsapp::MessageKey};

pub struct ReactionBuilder<'a> {
    pub ctx: &'a Context,
    pub reaction: &'a str,
}

impl<'a> ReactionBuilder<'a> {
    pub async fn send(self) -> anyhow::Result<()> {
        let chat = self.ctx.info.source.chat.clone();
        let sender = self.ctx.info.source.sender.clone();
        let id = self.ctx.info.id.clone();

        let key = MessageKey {
            from_me: Some(false),
            remote_jid: Some(chat.to_string()),
            id: Some(id),
            participant: Some(sender.to_string()),
        };

        self.ctx
            .wa_client
            .send_reaction(&chat, key, self.reaction)
            .await?;
        Ok(())
    }
}

impl<'a> IntoFuture for ReactionBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
