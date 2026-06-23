use crate::context::Context;
use std::pin::Pin;
use whatsapp_rust::waproto::whatsapp::MessageKey;

pub struct ReactionBuilder<'a> {
    pub ctx: &'a Context,
    pub reaction: &'a str,
}

impl<'a> ReactionBuilder<'a> {
    pub async fn send(self) -> anyhow::Result<()> {
        let key = MessageKey {
            remote_jid: Some(self.ctx.info().chat_jid().to_string()),
            participant: Some(self.ctx.info().sender_jid().to_string()),
            id: Some(self.ctx.msg_ctx.info.id.to_string()),
            from_me: Some(false),
        };

        self.ctx
            .msg_ctx
            .client
            .send_reaction(&self.ctx.info().chat_jid(), key, self.reaction)
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
