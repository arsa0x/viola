use super::Context;
use anyhow::Result;
use whatsapp_rust::waproto::whatsapp::{self, MessageKey, message::ExtendedTextMessage};

impl Context {
    pub async fn reply_text(&self, text: &str) -> Result<()> {
        let reply = whatsapp::Message {
            extended_text_message: Some(Box::new(ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(self.ctx_info())),
                ..Default::default()
            })),
            ..Default::default()
        };

        self.msg_ctx
            .client
            .send_message(self.chat_jid(), reply)
            .await?;

        Ok(())
    }

    pub async fn reply(&self, message: whatsapp::Message) -> Result<()> {
        self.msg_ctx
            .client
            .send_message(self.chat_jid(), message)
            .await?;
        Ok(())
    }

    pub async fn react(&self, emoji: &str) -> Result<()> {
        let key = MessageKey {
            remote_jid: Some(self.chat_jid().to_string()),
            participant: Some(self.sender_jid().to_string()),
            id: Some(self.msg_ctx.info.id.to_string()),
            from_me: Some(false),
        };

        self.msg_ctx
            .client
            .send_reaction(&self.chat_jid(), key, emoji)
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
        self.reply_text(&format!("Error:\n{}", err)).await
    }
}
