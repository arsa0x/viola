pub mod cta_url;
pub mod inapp_signup;
pub mod quick_reply;
pub mod single_select;

use crate::context::Context;
use std::pin::Pin;
use whatsapp_rust::waproto::whatsapp::{
    self,
    message::{
        InteractiveMessage,
        interactive_message::{self, Body, Header},
    },
};

pub struct InteractiveBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub body: Option<Body>,
    pub header: Option<Box<Header>>,
    pub interactive: Option<interactive_message::InteractiveMessage>,
}

impl<'a> InteractiveBuilder<'a> {
    pub fn body(mut self, body: Body) -> Self {
        self.body = Some(body);
        self
    }

    pub fn header(mut self, header: Header) -> Self {
        self.header = Some(Box::new(header));
        self
    }

    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let quoted = if self.quoted {
            Some(Box::new(self.ctx.info().ctx_info()))
        } else {
            None
        };

        self.ctx
            .message()
            .raw(whatsapp::Message {
                interactive_message: Some(Box::new(InteractiveMessage {
                    header: self.header,
                    body: self.body,
                    interactive_message: self.interactive,
                    context_info: quoted,
                    ..Default::default()
                })),
                ..Default::default()
            })
            .await
    }
}

impl<'a> IntoFuture for InteractiveBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
