use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{Pixel, input};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{self, Flags};
use ffmpeg_next::util::frame::Video as VideoFrame;
use std::io::Cursor;
use std::sync::OnceLock;
use whatsapp_rust::{wacore::proto_helpers::MessageExt, waproto::whatsapp::Message};

static FFMPEG_INIT: OnceLock<()> = OnceLock::new();

pub fn init_ffmpeg() {
    FFMPEG_INIT.get_or_init(|| ffmpeg::init().expect("failed to init ffmpeg"));
}

pub fn get_text_content(msg: &Message) -> Option<&str> {
    if let Some(once) = &msg.view_once_message {
        return once.message.as_deref().and_then(get_text_content);
    }

    if let Some(once_v2) = &msg.view_once_message_v2 {
        return once_v2.message.as_deref().and_then(get_text_content);
    }

    if let Some(text) = &msg.text_content() {
        return Some(text);
    }

    if let Some(image) = &msg.image_message {
        if let Some(caption) = &image.caption {
            return Some(caption);
        }
    }

    if let Some(video) = &msg.video_message {
        if let Some(caption) = &video.caption {
            return Some(caption);
        }
    }

    if let Some(document) = &msg.document_message {
        if let Some(caption) = &document.caption {
            return Some(caption);
        }
    }

    None
}

pub fn generate_video_thumbnail(video_path: &str) -> anyhow::Result<Vec<u8>> {
    init_ffmpeg();

    let mut ictx = input(&video_path)?;

    let input_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or_else(|| anyhow::anyhow!("cannot find video stream fron this file"))?;

    let video_stream_index = input_stream.index();

    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    let mut rgb_buffer = Vec::new();
    let mut width = 0;
    let mut height = 0;

    'packet_loop: for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;

            let mut decoded_frame = VideoFrame::empty();

            if decoder.receive_frame(&mut decoded_frame).is_ok() {
                width = decoded_frame.width();
                height = decoded_frame.height();

                let mut scaler = scaling::context::Context::get(
                    decoder.format(),
                    width,
                    height,
                    Pixel::RGB24,
                    width,
                    height,
                    Flags::BILINEAR,
                )?;

                let mut rgb_frame = VideoFrame::empty();
                scaler.run(&decoded_frame, &mut rgb_frame)?;

                rgb_buffer = rgb_frame.data(0).to_vec();

                break 'packet_loop;
            }
        }
    }

    if rgb_buffer.is_empty() {
        return Err(anyhow::anyhow!("failed to get frame from video"));
    }

    let img_buffer = image::RgbImage::from_raw(width, height, rgb_buffer)
        .ok_or_else(|| anyhow::anyhow!("failed to create buffer image from raw rgb"))?;

    let dynamic_img = image::DynamicImage::ImageRgb8(img_buffer);
    let thumbnail = dynamic_img.thumbnail(100, 100);

    let mut jpeg_bytes = Cursor::new(Vec::new());
    thumbnail.write_to(&mut jpeg_bytes, image::ImageFormat::Jpeg)?;

    Ok(jpeg_bytes.into_inner())
}
