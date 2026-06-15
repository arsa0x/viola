use crate::utils;
use std::io::Cursor;

pub struct Thumbnail;

impl Thumbnail {
    pub fn image_thumbnail_from_memory(media_bytes: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory(media_bytes).ok()?;
        let thumbnail = img.thumbnail(100, 100);

        let mut jpeg_bytes = Cursor::new(Vec::new());

        thumbnail
            .write_to(&mut jpeg_bytes, image::ImageFormat::Jpeg)
            .ok()?;
        Some(jpeg_bytes.into_inner())
    }

    pub fn video_thumbnail(media_bytes: &Vec<u8>) -> Option<Vec<u8>> {
        let temp_path = tempfile::Builder::new()
            .prefix("thumb_")
            .suffix(".mp4")
            .tempdir_in(dirs::home_dir()?.join("viola").join("cache"))
            .expect("failed to found cache dir");

        if std::fs::write(&temp_path, media_bytes).is_ok() {
            let res = utils::generate_video_thumbnail(&temp_path.path().to_string_lossy()).ok();
            let _ = std::fs::remove_file(temp_path.path());
            res
        } else {
            None
        }
    }
}
