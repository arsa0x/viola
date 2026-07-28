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

pub struct CtaUrlBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub header: Header,
    pub body: Body,
    pub footer: Footer,
    pub header_media: Option<HeaderMediaInput>,
    pub footer_media: Option<FooterMediaInput>,
    pub cta: Vec<CtaButton>,
}

pub struct CtaButton {
    pub display_text: String,
    pub id: String,
    pub url: String,
    pub merchant_url: String,
}

impl<'a> CtaUrlBuilder<'a> {
    pub fn new(ctx: &'a Context, cta: Vec<CtaButton>) -> Self {
        Self {
            ctx,
            cta,
            quoted: false,
            header: Header::default(),
            body: Body::default(),
            footer: Footer::default(),
            header_media: None,
            footer_media: None,
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

        let native_flow = interactive_message::InteractiveMessage::NativeFlowMessage(Box::new(
            NativeFlowMessage {
                message_params_json: Some("{}".into()),
                message_version: Some(1),
                buttons: self
                    .cta
                    .iter()
                    .map(|c| NativeFlowButton {
                        name: Some("cta_url".into()),
                        button_params_json: Some(
                            serde_json::json!({
                                "display_text": c.display_text,
                                "id": c.id,
                                "url": c.url,
                                "merchant_url": c.merchant_url,
                            })
                            .to_string(),
                        ),
                    })
                    .collect(),
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

sendable_builder!(CtaUrlBuilder);
