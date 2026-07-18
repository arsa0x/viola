use std::sync::Arc;

use whatsapp_rust::{
    anyhow::{self, anyhow},
    download::{Downloadable, MediaType},
    proto_helpers::build_quote_context,
    types::message::MessageInfo,
    waproto::whatsapp::{self, ContextInfo},
};

use crate::{http::Http, message::MessageFactory};

pub struct Context {
    pub http_client: isahc::HttpClient,
    pub wa_client: Arc<whatsapp_rust::Client>,
    pub message: Arc<whatsapp::Message>,
    pub info: Arc<MessageInfo>,
    pub args: Vec<String>,
}

impl Context {
    pub fn send(&self) -> MessageFactory<'_> {
        MessageFactory { ctx: self }
    }

    pub fn http(&self) -> Http<'_> {
        Http { ctx: self }
    }

    pub fn build_ctx_info(&self) -> ContextInfo {
        build_quote_context(
            self.info.id.clone(),
            self.info.source.sender.clone(),
            &self.message.clone(),
        )
    }

    pub fn get_current_media(&self) -> anyhow::Result<(MediaType, &'_ dyn Downloadable)> {
        Self::extract_media(&self.message)
    }

    pub fn get_quoted_media(&self) -> anyhow::Result<(MediaType, &'_ dyn Downloadable)> {
        let msg = &self.message;

        let ext = msg
            .extended_text_message
            .as_option()
            .ok_or_else(|| anyhow::anyhow!("not a reply message"))?;

        let quoted = ext
            .context_info
            .as_option()
            .and_then(|ctx| ctx.quoted_message.as_option())
            .ok_or_else(|| anyhow::anyhow!("quoted message not found"))?;

        Self::extract_media(quoted)
    }

    fn extract_media(msg: &whatsapp::Message) -> anyhow::Result<(MediaType, &'_ dyn Downloadable)> {
        if let Some(vo) = msg.view_once_message.as_option() {
            if let Some(inner) = vo.message.as_option() {
                return Self::extract_media(inner);
            }
        }

        if let Some(vo) = msg.view_once_message_v2.as_option() {
            if let Some(inner) = vo.message.as_option() {
                return Self::extract_media(inner);
            }
        }

        if let Some(img) = msg.image_message.as_option() {
            return Ok((MediaType::Image, img));
        }

        if let Some(video) = msg.video_message.as_option() {
            return Ok((MediaType::Video, video));
        }

        if let Some(audio) = msg.audio_message.as_option() {
            return Ok((MediaType::Audio, audio));
        }

        if let Some(sticker) = msg.sticker_message.as_option() {
            return Ok((MediaType::Sticker, sticker));
        }

        if let Some(doc) = msg.document_message.as_option() {
            return Ok((MediaType::Document, doc));
        }

        Err(anyhow!("quoted message does not contain media"))
    }
}
