use anyhow::{Context as _, Result, anyhow};
use ffmpeg_next as ffmpeg;
use image::{DynamicImage, ImageBuffer, Rgb, imageops::FilterType};
use std::io::Write;

pub const DEFAULT_MAX_DIM: u32 = 96;

const JPEG_QUALITY: u8 = 70;

pub fn init_ffmpeg() -> Result<()> {
    ffmpeg::init().map_err(|e| anyhow!("failed to initialize ffmpeg: {e}"))
}

pub async fn video_thumbnail_async(bytes: &[u8], max_dim: Option<u32>) -> Result<Vec<u8>> {
    let bytes = bytes.to_vec();
    let max_dim = max_dim.unwrap_or(DEFAULT_MAX_DIM);
    tokio::task::spawn_blocking(move || video_thumbnail(&bytes, max_dim))
        .await
        .context("video thumbnail task panicked")?
}

pub fn video_thumbnail(bytes: &[u8], max_dim: u32) -> Result<Vec<u8>> {
    let mut tmp = tempfile::NamedTempFile::new().context("failed to create temp file")?;
    tmp.write_all(bytes)
        .context("failed to write video bytes to temp file")?;
    tmp.flush().context("failed to flush temp file")?;

    let mut ictx = ffmpeg::format::input(&tmp.path()).context("failed to open video")?;

    let input_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or_else(|| anyhow!("no video stream found in input"))?;
    let video_stream_index = input_stream.index();

    let decoder_ctx = ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())
        .context("failed to build decoder context from stream parameters")?;
    let mut decoder = decoder_ctx
        .decoder()
        .video()
        .context("failed to open video decoder")?;

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )
    .context("failed to create pixel-format scaler")?;

    let mut rgb_frame = ffmpeg::util::frame::Video::empty();
    let mut got_frame = false;

    'decode: for (stream, packet) in ictx.packets() {
        if stream.index() != video_stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .context("failed to send packet to decoder")?;

        let mut frame = ffmpeg::util::frame::Video::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            scaler
                .run(&frame, &mut rgb_frame)
                .context("failed to scale decoded frame")?;
            got_frame = true;
            break 'decode;
        }
    }

    if !got_frame {
        let _ = decoder.send_eof();
        let mut frame = ffmpeg::util::frame::Video::empty();
        if decoder.receive_frame(&mut frame).is_ok() {
            scaler
                .run(&frame, &mut rgb_frame)
                .context("failed to scale flushed frame")?;
            got_frame = true;
        }
    }

    if !got_frame {
        return Err(anyhow!("could not decode any frame from the video"));
    }

    let width = rgb_frame.width();
    let height = rgb_frame.height();
    let stride = rgb_frame.stride(0);
    let data = rgb_frame.data(0);

    let mut packed = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        let row_start = y as usize * stride;
        let row_end = row_start + width as usize * 3;
        packed.extend_from_slice(&data[row_start..row_end]);
    }

    let img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, packed)
        .ok_or_else(|| anyhow!("decoded frame dimensions did not match pixel buffer size"))?;

    encode_thumbnail(DynamicImage::ImageRgb8(img_buffer), max_dim)
}

pub async fn image_thumbnail_async(bytes: &[u8], max_dim: Option<u32>) -> Result<Vec<u8>> {
    let bytes = bytes.to_vec();
    let max_dim = max_dim.unwrap_or(DEFAULT_MAX_DIM);
    tokio::task::spawn_blocking(move || image_thumbnail(&bytes, max_dim))
        .await
        .context("image thumbnail task panicked")?
}

pub fn image_thumbnail(bytes: &[u8], max_dim: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("failed to decode image")?;
    encode_thumbnail(img, max_dim)
}

fn encode_thumbnail(img: DynamicImage, max_dim: u32) -> Result<Vec<u8>> {
    let thumb = img.resize(max_dim, max_dim, FilterType::Triangle);
    let rgb = thumb.to_rgb8();

    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder
        .encode_image(&rgb)
        .context("failed to encode JPEG thumbnail")?;

    Ok(out)
}
