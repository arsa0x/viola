use whatsapp_rust::{
    anyhow,
    waproto::whatsapp::message::interactive_message::{footer, header},
};

use crate::{
    Context,
    message::{AudioBuilder, DocumentBuilder, ImageBuilder, MediaSource, VideoBuilder},
};

#[derive(Debug)]
pub enum HeaderMediaInput {
    Image(MediaSource),
    Video(MediaSource),
    Document(MediaSource),
    Thumbnail(Vec<u8>),
    Raw(header::Media),
}

impl<'a> HeaderMediaInput {
    pub async fn resolve(self, ctx: &'a Context) -> anyhow::Result<header::Media> {
        Ok(match self {
            HeaderMediaInput::Image(source) => {
                let msg = ImageBuilder {
                    ctx,
                    source,
                    caption: None,
                    thumbnail: None,
                    quoted: false,
                }
                .into_message()
                .await?
                .image_message
                .into_option()
                .ok_or_else(|| anyhow::anyhow!("upload does not produce image_message"))?;
                header::Media::ImageMessage(Box::new(msg))
            }
            HeaderMediaInput::Video(source) => {
                let msg = VideoBuilder {
                    ctx,
                    source,
                    caption: None,
                    thumbnail: None,
                    quoted: false,
                }
                .into_message()
                .await?
                .video_message
                .into_option()
                .ok_or_else(|| anyhow::anyhow!("upload does not produce video_message"))?;
                header::Media::VideoMessage(Box::new(msg))
            }
            HeaderMediaInput::Document(source) => {
                let msg = DocumentBuilder {
                    ctx,
                    source,
                    caption: None,
                    thumbnail: None,
                    quoted: false,
                }
                .into_message()
                .await?
                .document_message
                .into_option()
                .ok_or_else(|| anyhow::anyhow!("upload does not produce document_message"))?;
                header::Media::DocumentMessage(Box::new(msg))
            }
            HeaderMediaInput::Thumbnail(bytes) => header::Media::JpegThumbnail(bytes),
            HeaderMediaInput::Raw(media) => media,
        })
    }
}

#[derive(Debug)]
pub enum FooterMediaInput {
    Audio(MediaSource),
    Raw(footer::Media),
}

impl<'a> FooterMediaInput {
    pub async fn resolve(self, ctx: &'a Context) -> anyhow::Result<footer::Media> {
        Ok(match self {
            FooterMediaInput::Audio(source) => {
                let msg = AudioBuilder {
                    ctx,
                    source,
                    quoted: false,
                    ptt: None,
                }
                .into_message()
                .await?
                .audio_message
                .into_option()
                .ok_or_else(|| anyhow::anyhow!("upload does not produce audio_message"))?;
                footer::Media::AudioMessage(Box::new(msg))
            }
            FooterMediaInput::Raw(media) => media,
        })
    }
}

macro_rules! header_media_setters {
    () => {
        pub fn header_image(mut self, source: crate::message::media::MediaSource) -> Self {
            self.header_media = Some(crate::message::interactive::media::HeaderMediaInput::Image(
                source,
            ));
            self
        }
        pub fn header_video(mut self, source: crate::message::media::MediaSource) -> Self {
            self.header_media = Some(crate::message::interactive::media::HeaderMediaInput::Video(
                source,
            ));
            self
        }
        pub fn header_document(mut self, source: crate::message::media::MediaSource) -> Self {
            self.header_media =
                Some(crate::message::interactive::media::HeaderMediaInput::Document(source));
            self
        }
        pub fn header_thumbnail(mut self, bytes: Vec<u8>) -> Self {
            self.header_media =
                Some(crate::message::interactive::media::HeaderMediaInput::Thumbnail(bytes));
            self
        }
        pub fn header_media_raw(
            mut self,
            media: whatsapp_rust::waproto::whatsapp::message::interactive_message::header::Media,
        ) -> Self {
            self.header_media = Some(crate::message::interactive::media::HeaderMediaInput::Raw(
                media,
            ));
            self
        }
    };
}

macro_rules! footer_media_setters {
    () => {
        pub fn footer_audio(mut self, source: crate::message::media::MediaSource) -> Self {
            self.footer_media = Some(crate::message::interactive::media::FooterMediaInput::Audio(
                source,
            ));
            self
        }
        pub fn footer_media_raw(
            mut self,
            media: whatsapp_rust::waproto::whatsapp::message::interactive_message::footer::Media,
        ) -> Self {
            self.footer_media = Some(crate::message::interactive::media::FooterMediaInput::Raw(
                media,
            ));
            self
        }
    };
}

pub(crate) use {footer_media_setters, header_media_setters};
