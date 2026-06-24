use crate::Context;
use std::pin::Pin;
use whatsapp_rust::waproto::whatsapp::message::interactive_message::{
    self, NativeFlowMessage, native_flow_message::NativeFlowButton,
};

pub struct QuickReplyBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub title: Option<String>,
    pub text_body: Option<String>,
    pub footer: Option<String>,
    pub buttons: Vec<QuickReplyButton>,
}

pub struct QuickReplyButton {
    pub text: String,
    pub id: String,
}

impl<'a> QuickReplyBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn text_body(mut self, text: impl Into<String>) -> Self {
        self.text_body = Some(text.into());
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let message = self
            .ctx
            .send()
            .interactive()
            .raw(interactive_message::InteractiveMessage::NativeFlowMessage(
                NativeFlowMessage {
                    message_params_json: Some("{}".into()),
                    message_version: Some(1),
                    buttons: self
                        .buttons
                        .iter()
                        .map(|q| NativeFlowButton {
                            name: Some("quick_reply".into()),
                            button_params_json: Some(
                                serde_json::json!({
                                    "display_text": q.text,
                                    "id": q.id,
                                })
                                .to_string(),
                            ),
                        })
                        .collect(),
                },
            ))
            .body(interactive_message::Body {
                text: self.text_body,
            })
            .header(interactive_message::Header {
                title: self.title,
                has_media_attachment: Some(false),
                ..Default::default()
            })
            .footer(interactive_message::Footer {
                text: self.footer,
                has_media_attachment: None,
                ..Default::default()
            });
        if self.quoted {
            message.quoted().await
        } else {
            message.await
        }
    }
}

impl<'a> IntoFuture for QuickReplyBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
