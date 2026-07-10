use whatsapp_rust::{Jid, waproto::whatsapp::ContextInfo};

use crate::{config::Config, context::Context};

pub struct Info<'a> {
    pub ctx: &'a Context,
}

impl<'a> Info<'a> {
    pub fn chat_jid(&self) -> Jid {
        self.ctx.msg_ctx.info.source.chat.clone()
    }

    pub fn sender_jid(&self) -> Jid {
        self.ctx.msg_ctx.info.source.sender.clone()
    }

    pub fn ctx_info(&self) -> ContextInfo {
        self.ctx.msg_ctx.build_quote_context()
    }

    pub fn processing_ms(&self) -> f64 {
        self.ctx.created_at.elapsed().as_secs_f64() * 1000.0
    }

    pub fn is_group(&self) -> bool {
        self.ctx.msg_ctx.info.source.is_group
    }

    pub fn is_owner(&self, config: &Config) -> bool {
        let sender = match self.sender_str() {
            Ok(s) => s,
            Err(_) => return false,
        };
        config.bot.owners.iter().any(|o| *o == sender)
    }

    pub fn sender_str(&self) -> anyhow::Result<String> {
        let phone = self
            .ctx
            .msg_ctx
            .info
            .source
            .sender_alt
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Sender alternative info missing"))?
            .user_base();

        Ok(phone.to_string())
    }
}
