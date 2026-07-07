use std::pin::Pin;

use whatsapp_rust::waproto::whatsapp::message::interactive_message::{
    self, NativeFlowMessage, native_flow_message::NativeFlowButton,
};

use crate::Context;

pub struct InappSignupBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub title: Option<String>,
    pub text_body: Option<String>,
}

impl<'a> InappSignupBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub async fn send(self) -> anyhow::Result<()> {
        let message = self
            .ctx
            .send()
            .interactive()
            .raw(interactive_message::InteractiveMessage::NativeFlowMessage(
                Box::new(NativeFlowMessage {
                    message_params_json: Some("{}".into()),
                    message_version: Some(1),
                    buttons: vec![NativeFlowButton {
                        button_params_json: Some("{}".into()),
                        name: Some("inapp_signup".into()),
                    }],
                }),
            ))
            .body(interactive_message::Body {
                text: self.text_body,
            })
            .header(interactive_message::Header {
                title: self.title,
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

impl<'a> IntoFuture for InappSignupBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
