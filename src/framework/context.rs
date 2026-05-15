use std::{sync::Arc, time::Instant};

use anyhow::anyhow;
use wacore::{proto_helpers::MessageExt, types::message::MessageInfo};
use waproto::whatsapp;
use whatsapp_rust::{Client, bot::MessageContext};

use crate::utils::config::Config;

// use crate::framework::state::AppState;

#[derive(Clone)]
pub struct Context {
    pub msg: MessageContext,
    pub text: String,
    pub command: String,
    pub args: Vec<String>,
    // pub state: Arc<AppState>,
    pub start: Instant,
    pub config: Arc<Config>,
}

impl Context {
    pub fn new(
        message: &Arc<waproto::whatsapp::Message>,
        info: &Arc<MessageInfo>,
        client: Arc<Client>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            msg: MessageContext::from_parts(message, info, client),
            text: String::new(),
            command: String::new(),
            args: Vec::new(),
            start: Instant::now(),
            config,
        }
    }
    pub async fn reply(&self, text: &str) -> anyhow::Result<()> {
        let ctx_info = self.msg.build_quote_context();
        let reply = whatsapp::Message {
            extended_text_message: Some(Box::new(whatsapp::message::ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(ctx_info)),
                ..Default::default()
            })),
            ..Default::default()
        };

        if let Err(e) = self.msg.send_message(reply).await {
            println!("failed to send message: {}", e);
        }
        Ok(())
    }

    pub fn sender(&self) -> anyhow::Result<String> {
        let phone = self
            .msg
            .info
            .source
            .sender_alt
            .as_ref()
            .ok_or_else(|| anyhow!("Sender alternative info missing"))?
            .user_base();

        Ok(phone.to_string())
    }

    pub fn is_group(&self) -> bool {
        self.msg.info.source.is_group
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    pub fn content(&self) -> Option<String> {
        let msg = &self.msg.message;

        if let Some(text) = msg.text_content() {
            return Some(text.to_string());
        }

        if let Some(image) = &msg.image_message {
            if let Some(caption) = &image.caption {
                return Some(caption.to_string());
            }
        }

        if let Some(video) = &msg.video_message {
            if let Some(caption) = &video.caption {
                return Some(caption.to_string());
            }
        }

        if let Some(document) = &msg.document_message {
            if let Some(caption) = &document.caption {
                return Some(caption.to_string());
            }
        }

        None
    }

    pub fn parse_command(mut self, prefix: &str) -> Self {
        if let Some(text) = self.content() {
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
