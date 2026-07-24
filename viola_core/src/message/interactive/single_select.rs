use whatsapp_rust::{
    anyhow,
    buffa::MessageField,
    serde_json,
    waproto::whatsapp::{
        self,
        message::{
            InteractiveMessage,
            interactive_message::{
                self, Body, Footer, Header, NativeFlowMessage,
                native_flow_message::NativeFlowButton,
            },
        },
    },
};

use crate::{
    Context,
    message::{
        context_info_slot,
        interactive::media::{
            FooterMediaInput, HeaderMediaInput, footer_media_setters, header_media_setters,
        },
        sendable_builder,
    },
};

pub struct SingleSelectBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub header: Header,
    pub body: Body,
    pub footer: Footer,
    pub header_media: Option<HeaderMediaInput<'a>>,
    pub footer_media: Option<FooterMediaInput<'a>>,
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
    pub fn new(ctx: &'a Context, sections: Vec<SingleSelectSection>) -> Self {
        Self {
            ctx,
            quoted: false,
            header: Header::default(),
            body: Body::default(),
            footer: Footer::default(),
            footer_media: None,
            header_media: None,
            select_label: None,
            sections,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.header.title = Some(title.into());
        self
    }
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer.text = Some(footer.into());
        self
    }
    pub fn text_body(mut self, text: impl Into<String>) -> Self {
        self.body.text = Some(text.into());
        self
    }
    pub fn select_label(mut self, label: impl Into<String>) -> Self {
        self.select_label = Some(label.into());
        self
    }
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    header_media_setters!();

    footer_media_setters!();

    pub async fn into_message(mut self) -> anyhow::Result<whatsapp::Message> {
        if let Some(input) = self.header_media {
            self.header.media = Some(input.resolve(self.ctx).await?);
            self.header.has_media_attachment = Some(true);
        }
        if let Some(input) = self.footer_media {
            self.footer.media = Some(input.resolve(self.ctx).await?);
            self.footer.has_media_attachment = Some(true);
        }

        let params = serde_json::json!({
            "title": self.select_label.unwrap_or_else(|| "Select Options".into()),
            "sections": self.sections.iter().map(|s| serde_json::json!({
                "title": s.title,
                "rows": s.rows.iter().map(|r| serde_json::json!({
                    "id": r.id, "title": r.title, "description": r.description
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
        });

        let native_flow = interactive_message::InteractiveMessage::NativeFlowMessage(Box::new(
            NativeFlowMessage {
                message_params_json: Some("{}".into()),
                message_version: Some(1),
                buttons: vec![NativeFlowButton {
                    name: Some("single_select".into()),
                    button_params_json: Some(params.to_string()),
                }],
            },
        ));

        Ok(whatsapp::Message {
            interactive_message: MessageField::some(InteractiveMessage {
                header: MessageField::some(self.header),
                body: MessageField::some(self.body),
                footer: MessageField::some(self.footer),
                interactive_message: Some(native_flow),
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

sendable_builder!(SingleSelectBuilder);
