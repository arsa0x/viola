use whatsapp_rust::{anyhow, waproto::whatsapp};

use crate::{
    Context,
    message::{context_info_slot, media::MediaSource},
};

pub struct AudioBuilder<'a> {
    pub ctx: &'a Context,
    pub source: MediaSource<'a>,
    pub quoted: bool,
    pub ptt: Option<bool>,
}

impl<'a> AudioBuilder<'a> {
    pub fn quoted(mut self) -> Self {
        self.quoted = true;
        self
    }

    pub fn ptt(mut self) -> Self {
        self.ptt = Some(true);
        self
    }

    pub async fn into_message(self) -> anyhow::Result<whatsapp::Message> {
        let bytes = self.source.get_media_bytes(self.ctx).await?;
        let upload = self
            .ctx
            .wa_client
            .upload(
                bytes,
                whatsapp_rust::download::MediaType::Audio,
                Default::default(),
            )
            .await?;

        Ok(whatsapp_rust::media::audio_message(
            upload,
            whatsapp_rust::media::AudioOptions {
                ptt: self.ptt,
                context_info: context_info_slot(self.ctx, self.quoted),
                ..Default::default()
            },
        ))
    }
}
