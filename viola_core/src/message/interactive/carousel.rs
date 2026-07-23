use crate::{Context, message::media::MediaSource};
use std::pin::Pin;
use whatsapp_rust::{
    NodeBuilder, anyhow,
    buffa::MessageField,
    download::MediaType,
    serde_json,
    waproto::whatsapp::{
        self,
        message::{
            self as wa_message, ImageMessage,
            interactive_message::{
                self, Body, CarouselMessage, Footer, Header, NativeFlowMessage,
                native_flow_message::NativeFlowButton,
            },
        },
    },
};

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
    pub title: String,
    pub subtitle: Option<String>,
    pub body_text: String,
    pub image: MediaSource<'a>,
    pub buttons: Vec<CarouselButton>,
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
                "display_text": display_text,
                "url": url,
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
                "display_text": display_text,
                "phone_number": phone_number,
            }),
        ),
        CarouselButton::CtaCopy {
            display_text,
            copy_code,
            id,
        } => (
            "cta_copy",
            serde_json::json!({
                "display_text": display_text,
                "copy_code": copy_code,
                "id": id.clone().unwrap_or_else(|| copy_code.clone()),
            }),
        ),
        CarouselButton::QuickReply { display_text, id } => (
            "quick_reply",
            serde_json::json!({
                "display_text": display_text,
                "id": id,
            }),
        ),
        CarouselButton::SingleSelect { title, sections } => (
            "single_select",
            serde_json::json!({
                "title": title,
                "sections": sections.iter().map(|s| {
                    serde_json::json!({
                        "title": s.title,
                        "rows": s.rows.iter().map(|r| serde_json::json!({
                            "id": r.id,
                            "title": r.title,
                            "description": r.description,
                        })).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            }),
        ),
    };

    NativeFlowButton {
        name: Some(name.into()),
        button_params_json: Some(params.to_string()),
    }
}

pub struct CarouselBuilder<'a> {
    pub ctx: &'a Context,
    pub quoted: bool,
    pub body_text: String,
    pub footer_text: Option<String>,
    pub cards: Vec<CarouselCard<'a>>,
}

impl<'a> CarouselBuilder<'a> {
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer_text = Some(footer.into());
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

    pub async fn send(self) -> anyhow::Result<()> {
        let mut cards = Vec::with_capacity(self.cards.len());

        for card in self.cards {
            let image = card.image.get_media(self.ctx).await?;

            let upload = self
                .ctx
                .wa_client
                .upload(image, MediaType::Image, Default::default())
                .await?;

            let header = Header {
                title: Some(card.title.clone()),
                subtitle: card.subtitle.clone(),
                has_media_attachment: Some(true),
                media: Some(interactive_message::header::Media::ImageMessage(Box::new(
                    ImageMessage {
                        url: Some(upload.url.clone()),
                        file_sha256: Some(upload.file_sha256.to_vec()),
                        file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),
                        media_key: Some(upload.media_key.to_vec()),
                        media_key_timestamp: Some(upload.media_key_timestamp),
                        direct_path: Some(upload.direct_path.clone()),
                        file_length: Some(upload.file_length),
                        mimetype: Some("image/jpeg".to_string()),
                        ..Default::default()
                    },
                ))),
                ..Default::default()
            };

            let native_flow = interactive_message::InteractiveMessage::NativeFlowMessage(Box::new(
                NativeFlowMessage {
                    message_params_json: Some("{}".into()),
                    message_version: Some(1),
                    buttons: card.buttons.iter().map(build_native_flow_button).collect(),
                },
            ));

            cards.push(wa_message::InteractiveMessage {
                header: MessageField::some(header),
                body: MessageField::some(Body {
                    text: Some(card.body_text),
                }),
                interactive_message: Some(native_flow),
                ..Default::default()
            });
        }

        let quoted = if self.quoted {
            MessageField::some(self.ctx.build_ctx_info())
        } else {
            MessageField::none()
        };

        let carousel =
            interactive_message::InteractiveMessage::CarouselMessage(Box::new(CarouselMessage {
                cards,
                message_version: Some(1),
                // carousel_card_type: Some(CarouselCardType::ALBUM_IMAGE),
                ..Default::default()
            }));

        let message = wa_message::InteractiveMessage {
            body: MessageField::some(Body {
                text: Some(self.body_text),
            }),
            footer: MessageField::some(Footer {
                text: self.footer_text,
                ..Default::default()
            }),
            header: MessageField::some(Header {
                has_media_attachment: Some(false),
                ..Default::default()
            }),
            interactive_message: Some(carousel),
            context_info: quoted,
            ..Default::default()
        };

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

        let opt = whatsapp_rust::send::SendOptions {
            extra_stanza_nodes: vec![biz_node],
            ..Default::default()
        };

        self.ctx
            .wa_client
            .send_message_with_options(
                self.ctx.info.source.chat.clone(),
                whatsapp::Message {
                    interactive_message: MessageField::some(message),
                    ..Default::default()
                },
                opt,
            )
            .await?;
        Ok(())
    }
}

impl<'a> IntoFuture for CarouselBuilder<'a> {
    type Output = anyhow::Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send().await })
    }
}
