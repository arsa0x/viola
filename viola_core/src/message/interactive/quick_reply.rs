use std::pin::Pin;

use whatsapp_rust::waproto::whatsapp::message::interactive_message::{
    self, NativeFlowMessage, native_flow_message::NativeFlowButton,
};

use crate::Context;

pub struct QuickReplyBuilder<'a> {
    pub ctx: &'a Context,
    pub title: String,
    pub body: String,
    pub quoted: bool,
    pub buttons: Vec<ButtonParam<'a>>,
}

pub struct ButtonParam<'a> {
    pub display_text: &'a str,
    pub id: &'a str,
}

impl<'a> QuickReplyBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let message = self
            .ctx
            .message()
            .interactive(interactive_message::InteractiveMessage::NativeFlowMessage(
                NativeFlowMessage {
                    message_params_json: Some("{}".into()),
                    message_version: Some(1),
                    buttons: self
                        .buttons
                        .iter()
                        .map(|btn| NativeFlowButton {
                            name: Some("quick_reply".into()),
                            button_params_json: Some(
                                serde_json::json!({
                                    "display_text": btn.display_text,
                                    "id": btn.id,
                                })
                                .to_string(),
                            ),
                        })
                        .collect(),
                },
            ))
            .body(interactive_message::Body {
                text: Some(self.body),
            })
            .header(interactive_message::Header {
                title: Some(self.title),
                has_media_attachment: Some(false),
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
