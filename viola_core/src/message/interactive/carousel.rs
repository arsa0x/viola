use whatsapp_rust::{
    NodeBuilder, anyhow,
    buffa::MessageField,
    serde_json,
    waproto::whatsapp::{
        self,
        message::{
            InteractiveMessage,
            interactive_message::{
                self, Body, CarouselMessage, Footer, Header, NativeFlowMessage,
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
    },
};

pub struct CarouselBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub header: Header,
    pub body: Body,
    pub footer: Footer,
    pub header_media: Option<HeaderMediaInput>,
    pub footer_media: Option<FooterMediaInput>,
    pub cards: Vec<CarouselCard>,
}

#[derive(Debug)]
pub enum CarouselButton {
    CtaUrl {
        display_text: String,
        url: String,
        merchant_url: Option<String>,
    },
    CtaCall {
        display_text: String,
        phone_number: String,
    },
    CtaCopy {
        display_text: String,
        copy_code: String,
        id: Option<String>,
    },
    QuickReply {
        display_text: String,
        id: String,
    },
    SingleSelect {
        title: String,
        sections: Vec<CarouselSelectSection>,
    },
}

#[derive(Debug)]
pub struct CarouselSelectSection {
    pub title: String,
    pub rows: Vec<CarouselSelectRow>,
}

#[derive(Debug)]
pub struct CarouselSelectRow {
    pub title: String,
    pub description: Option<String>,
    pub id: String,
}

#[derive(Debug)]
pub struct CarouselCard {
    pub header: Header,
    pub body: Body,
    pub footer: Footer,
    pub header_media: Option<HeaderMediaInput>,
    pub footer_media: Option<FooterMediaInput>,
    pub buttons: Vec<CarouselButton>,
}

impl<'a> CarouselCard {
    pub fn new(body_text: impl Into<String>) -> Self {
        Self {
            body: Body {
                text: Some(body_text.into()),
            },
            footer: Footer::default(),
            header: Header::default(),
            header_media: None,
            footer_media: None,
            buttons: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.header.title = Some(title.into());
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.header.subtitle = Some(subtitle.into());
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer.text = Some(footer.into());
        self
    }

    pub fn button(mut self, button: CarouselButton) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn buttons(mut self, buttons: impl IntoIterator<Item = CarouselButton>) -> Self {
        self.buttons.extend(buttons);
        self
    }

    header_media_setters!();

    footer_media_setters!();
}

fn build_native_flow_button(btn: &CarouselButton) -> NativeFlowButton {
    let (name, params) = match btn {
        CarouselButton::CtaUrl {
            display_text,
            url,
            merchant_url,
        } => (
            "cta_url",
            serde_json::json!({
                "display_text": display_text, "url": url,
                "merchant_url": merchant_url.clone().unwrap_or_else(|| url.clone()),
                "webview_interaction": true,
            }),
        ),
        CarouselButton::CtaCall {
            display_text,
            phone_number,
        } => (
            "cta_call",
            serde_json::json!({
                "display_text": display_text, "phone_number": phone_number,
            }),
        ),
        CarouselButton::CtaCopy {
            display_text,
            copy_code,
            id,
        } => (
            "cta_copy",
            serde_json::json!({
                "display_text": display_text, "copy_code": copy_code,
                "id": id.clone().unwrap_or_else(|| copy_code.clone()),
            }),
        ),
        CarouselButton::QuickReply { display_text, id } => (
            "quick_reply",
            serde_json::json!({
                "display_text": display_text, "id": id,
            }),
        ),
        CarouselButton::SingleSelect { title, sections } => (
            "single_select",
            serde_json::json!({
                "title": title,
                "sections": sections.iter().map(|s| serde_json::json!({
                    "title": s.title,
                    "rows": s.rows.iter().map(|r| serde_json::json!({
                        "id": r.id, "title": r.title, "description": r.description,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
        ),
    };
    NativeFlowButton {
        name: Some(name.into()),
        button_params_json: Some(params.to_string()),
    }
}

impl<'a> CarouselBuilder<'a> {
    pub fn new(ctx: &'a Context, body_text: impl Into<String>) -> Self {
        Self {
            ctx,
            quoted: false,
            header: Header::default(),
            body: Body {
                text: Some(body_text.into()),
            },
            footer: Footer::default(),
            footer_media: None,
            header_media: None,
            cards: Vec::new(),
        }
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer.text = Some(footer.into());
        self
    }
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn card(mut self, card: CarouselCard) -> Self {
        self.cards.push(card);
        self
    }

    pub fn cards(mut self, cards: impl IntoIterator<Item = CarouselCard>) -> Self {
        self.cards.extend(cards);
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

        let mut cards = Vec::with_capacity(self.cards.len());

        for mut card in self.cards {
            if let Some(input) = card.header_media.take() {
                card.header.media = Some(input.resolve(self.ctx).await?);
                card.header.has_media_attachment = Some(true);
            }
            if let Some(input) = card.footer_media.take() {
                card.footer.media = Some(input.resolve(self.ctx).await?);
                card.footer.has_media_attachment = Some(true);
            }

            let native_flow = interactive_message::InteractiveMessage::NativeFlowMessage(Box::new(
                NativeFlowMessage {
                    message_params_json: Some("{}".into()),
                    message_version: Some(1),
                    buttons: card.buttons.iter().map(build_native_flow_button).collect(),
                },
            ));

            cards.push(InteractiveMessage {
                header: MessageField::some(card.header),
                body: MessageField::some(card.body),
                footer: MessageField::some(card.footer),
                interactive_message: Some(native_flow),
                ..Default::default()
            });
        }

        let carousel =
            interactive_message::InteractiveMessage::CarouselMessage(Box::new(CarouselMessage {
                cards,
                message_version: Some(1),
                ..Default::default()
            }));

        Ok(whatsapp::Message {
            interactive_message: MessageField::some(InteractiveMessage {
                body: MessageField::some(self.body),
                footer: MessageField::some(self.footer),
                header: MessageField::some(self.header),
                interactive_message: Some(carousel),
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let ctx = self.ctx;

        let nodes = vec![
            NodeBuilder::new("biz")
                .children([NodeBuilder::new("interactive")
                    .attr("type", "native_flow")
                    .attr("v", "1")
                    .children([NodeBuilder::new("native_flow")
                        .attr("v", "9")
                        .attr("name", "mixed")
                        .build()])
                    .build()])
                .build(),
        ];

        let message = self.into_message().await?;

        ctx.wa_client
            .send_message_with_options(
                ctx.info.source.chat.clone(),
                message,
                whatsapp_rust::SendOptions::default().with_extra_stanza_nodes(nodes),
            )
            .await?;
        Ok(())
    }
}

impl<'a> IntoFuture for CarouselBuilder<'a> {
    type Output = anyhow::Result<()>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}
