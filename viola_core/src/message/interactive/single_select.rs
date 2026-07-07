use crate::Context;
use std::pin::Pin;
use whatsapp_rust::waproto::whatsapp::message::interactive_message::{
    self, NativeFlowMessage, native_flow_message::NativeFlowButton,
};

pub struct SingleSelectBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub title: Option<String>,
    pub footer: Option<String>,
    pub text_body: Option<String>,
    pub select_label: Option<String>,
    pub sections: Vec<SingleSelectSection>,
}

pub struct SingleSelectSection {
    pub title: String,
    pub rows: Vec<SingleSelectRow>,
}

pub struct SingleSelectRow {
    pub title: String,
    pub description: String,
    pub id: String,
}

impl<'a> SingleSelectBuilder<'a> {
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

    pub fn select_label(mut self, label: impl Into<String>) -> Self {
        self.select_label = Some(label.into());
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
                    name: Some("single_select".into()),
                    button_params_json: Some(
                        serde_json::json!({
                            "title": self.select_label.unwrap_or_else(|| "Select Options".into()),
                            "sections": self.sections.iter().map(|section| {
                                serde_json::json!({
                                    "title": section.title,
                                    "rows": section.rows.iter().map(|row| {
                                        serde_json::json!({
                                            "id": row.id,
                                            "title": row.title,
                                            "description": row.description
                                        })
                                    }).collect::<Vec<_>>()
                                })
                            }).collect::<Vec<_>>()
                        })
                        .to_string(),
                    ),
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
            })
            .footer(interactive_message::Footer {
                text: self.footer,
                ..Default::default()
            });
        if self.quoted {
            message.quoted().await
        } else {
            message.await
        }
    }
}

impl<'a> IntoFuture for SingleSelectBuilder<'a> {
    type Output = anyhow::Result<()>;

    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
