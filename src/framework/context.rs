use std::sync::Arc;

use wacore::{proto_helpers::MessageExt, types::message::MessageInfo};
use waproto::whatsapp;
use whatsapp_rust::{Client, bot::MessageContext};

// use crate::framework::state::AppState;

#[derive(Clone)]
pub struct Context {
    pub msg_context: MessageContext,
    pub text: String,
    pub command: String,
    pub args: Vec<String>,
    // pub state: Arc<AppState>,
}

impl Context {
    pub fn new(
        message: &Arc<waproto::whatsapp::Message>,
        info: &Arc<MessageInfo>,
        client: Arc<Client>,
    ) -> Self {
        Self {
            msg_context: MessageContext::from_parts(message, info, client),
            text: String::new(),
            command: String::new(),
            args: Vec::new(),
        }
    }
    pub async fn reply(&self, text: &str) -> anyhow::Result<()> {
        let ctx_info = self.msg_context.build_quote_context();
        let reply = whatsapp::Message {
            extended_text_message: Some(Box::new(whatsapp::message::ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(ctx_info)),
                ..Default::default()
            })),
            ..Default::default()
        };

        if let Err(e) = self.msg_context.send_message(reply).await {
            println!("failed to send message: {}", e);
        }
        Ok(())
    }

    pub fn parse_command(mut self, prefix: &str) -> Self {
        if let Some(text) = self.msg_context.message.text_content() {
            self.text = text.to_string();

            if text.starts_with(prefix) {
                let without_prefix = text.trim_start_matches(prefix);

                let parts: Vec<String> = without_prefix
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                if let Some((cmd, args)) = parts.split_first() {
                    self.command = cmd.to_lowercase();
                    self.args = args.to_vec();
                }
            }
        }

        self
    }
}
