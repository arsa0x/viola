use crate::context::Context;
use anyhow::{Result, anyhow};
use whatsapp_rust::{Jid, waproto::whatsapp::ContextInfo};

impl Context {
    pub fn chat_jid(&self) -> Jid {
        self.msg_ctx.info.source.chat.clone()
    }

    pub fn sender_jid(&self) -> Jid {
        self.msg_ctx.info.source.sender.clone()
    }

    pub fn ctx_info(&self) -> ContextInfo {
        self.msg_ctx.build_quote_context()
    }

    pub fn processing_ms(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64() * 1000.0
    }

    pub fn is_group(&self) -> bool {
        self.msg_ctx.info.source.is_group
    }

    pub fn sender_str(&self) -> Result<String> {
        let phone = self
            .msg_ctx
            .info
            .source
            .sender_alt
            .as_ref()
            .ok_or_else(|| anyhow!("Sender alternative info missing"))?
            .user_base();

        Ok(phone.to_string())
    }
}
