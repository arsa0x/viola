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
    pub header_media: Option<HeaderMediaInput<'a>>,
    pub footer_media: Option<FooterMediaInput<'a>>,
    pub cards: Vec<CarouselCard<'a>>,
}

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

pub struct CarouselSelectSection {
    pub title: String,
    pub rows: Vec<CarouselSelectRow>,
}
pub struct CarouselSelectRow {
    pub title: String,
    pub description: Option<String>,
    pub id: String,
}

pub struct CarouselCard<'a> {
    pub header: Header,
    pub body: Body,
    pub footer: Footer,
    pub header_media: Option<HeaderMediaInput<'a>>,
    pub footer_media: Option<FooterMediaInput<'a>>,
    // pub title: String,
    // pub subtitle: Option<String>,
    // pub body_text: String,
    // pub image: MediaSource<'a>,
    pub buttons: Vec<CarouselButton>,
}

impl<'a> CarouselCard<'a> {
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
    pub fn card(mut self, card: CarouselCard<'a>) -> Self {
        self.cards.push(card);
        self
    }

    header_media_setters!();

    footer_media_setters!();

    pub async fn into_message(self) -> anyhow::Result<whatsapp::Message> {
        let mut cards = Vec::with_capacity(self.cards.len());

        for card in self.cards {
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
                header: MessageField::some(Header {
                    has_media_attachment: Some(false),
                    ..Default::default()
                }),
                interactive_message: Some(carousel),
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let ctx = self.ctx;

        let biz_node = NodeBuilder::new("biz")
            .children([NodeBuilder::new("interactive")
                .attr("type", "native_flow")
                .attr("v", "1")
                .children([NodeBuilder::new("native_flow")
                    .attr("v", "9")
                    .attr("name", "mixed")
                    .build()])
                .build()])
            .build();

        let message = self.into_message().await?;
        ctx.wa_client
            .send_message_with_options(
                ctx.info.source.chat.clone(),
                message,
                whatsapp_rust::SendOptions {
                    extra_stanza_nodes: vec![biz_node],
                    ..Default::default()
                },
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
