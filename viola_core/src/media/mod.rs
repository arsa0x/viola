pub mod thumbnail;

pub use thumbnail::{
    DEFAULT_MAX_DIM, image_thumbnail, image_thumbnail_async, init_ffmpeg, video_thumbnail,
    video_thumbnail_async,
};
