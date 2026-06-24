use crate::Context;
use std::pin::Pin;
use whatsapp_rust::waproto::whatsapp::{self, message::ExtendedTextMessage};

pub struct TextBuilder<'a> {
    pub ctx: &'a Context,
    pub text: String,
    pub quoted: bool,
}

impl<'a> TextBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }
    pub async fn send(self) -> anyhow::Result<()> {
        let message = if self.quoted {
            whatsapp::Message {
                extended_text_message: Some(Box::new(ExtendedTextMessage {
                    text: Some(self.text.into()),
                    context_info: Some(Box::new(self.ctx.info().ctx_info())),
                    ..Default::default()
                })),
                ..Default::default()
            }
        } else {
            whatsapp::Message {
                conversation: Some(self.text.into()),
                ..Default::default()
            }
        };

        self.ctx.send().raw(message).await?;

        Ok(())
    }
}

impl<'a> IntoFuture for TextBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
