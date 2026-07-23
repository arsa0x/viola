use image::{DynamicImage, imageops::FilterType};
use isahc::AsyncReadResponseExt;
use whatsapp_rust::{
    anyhow::{self, Context},
    buffa::MessageField,
    upload::UploadResponse,
    waproto::whatsapp::{
        self,
        message::{
            ImageMessage, InteractiveMessage,
            interactive_message::{
                Body, CarouselMessage, Footer, Header, NativeFlowMessage,
                native_flow_message::NativeFlowButton,
            },
        },
    },
};

pub const DEFAULT_MAX_DIM: u32 = 96;

const JPEG_QUALITY: u8 = 70;

pub fn image_thumbnail(bytes: &[u8], max_dim: u32) -> anyhow::Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("failed to decode image")?;
    encode_thumbnail(img, max_dim)
}

fn encode_thumbnail(img: DynamicImage, max_dim: u32) -> anyhow::Result<Vec<u8>> {
    let thumb = img.resize(max_dim, max_dim, FilterType::Triangle);
    let rgb = thumb.to_rgb8();

    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder
        .encode_image(&rgb)
        .context("failed to encode JPEG thumbnail")?;

    Ok(out)
}

fn build_card(upload: UploadResponse, thumbnail: Vec<u8>, url: &str) -> InteractiveMessage {
    InteractiveMessage {
        header: MessageField::some(Header {
            title: Some("elaina".into()),
            subtitle: Some(String::new()),
            has_media_attachment: Some(true),

            media: Some(
                ImageMessage {
                    url: Some(upload.url),
                    direct_path: Some(upload.direct_path),

                    media_key: Some(upload.media_key.to_vec()),
                    file_sha256: Some(upload.file_sha256.to_vec()),
                    file_enc_sha256: Some(upload.file_enc_sha256.to_vec()),

                    file_length: Some(upload.file_length),
                    media_key_timestamp: Some(upload.media_key_timestamp),
                    mimetype: Some("image/jpeg".into()),
                    jpeg_thumbnail: Some(thumbnail),

                    ..Default::default()
                }
                .into(),
            ),

            ..Default::default()
        }),

        body: MessageField::some(Body {
            text: Some(url.to_owned()),
            ..Default::default()
        }),

        footer: MessageField::some(Footer {
            text: Some(String::new()),
            ..Default::default()
        }),

        interactive_message: Some(
            NativeFlowMessage {
                buttons: vec![NativeFlowButton {
                    name: Some("cta_url".into()),
                    button_params_json: Some(format!(
                        r#"{{"display_text":"🔗 Open","url":"{}","webview_interaction":true}}"#,
                        url
                    )),
                }],

                message_params_json: Some("{}".into()),
                message_version: Some(1),
            }
            .into(),
        ),

        ..Default::default()
    }
}

#[viola_macros::command(
  triggers = ["c"],
  category = "tools")
]
async fn carousel(ctx: viola_core::Context) -> anyhow::Result<()> {
    let mut resp = ctx
        .http_client
        .get_async("http://localhost:8000/arona_.png")
        .await?;
    let bytes = resp.bytes().await?;
    let thumb = image_thumbnail(&bytes.as_slice(), DEFAULT_MAX_DIM)?;

    let upload = ctx
        .wa_client
        .upload(
            bytes,
            whatsapp_rust::download::MediaType::Image,
            Default::default(),
        )
        .await?;
    let card1 = build_card(
        upload,
        thumb,
        "https://www.pinterest.com/pin/594334482121370278/",
    );

    let outer = InteractiveMessage {
        header: MessageField::some(Header {
            has_media_attachment: Some(false),
            ..Default::default()
        }),

        body: MessageField::some(Body {
            text: Some("🖼 Pinterest — elaina".into()),
            ..Default::default()
        }),

        footer: MessageField::some(Footer {
            text: Some("48 results found".into()),
            ..Default::default()
        }),

        interactive_message: Some(
            CarouselMessage {
                cards: vec![card1],
                message_version: Some(1),
                carousel_card_type: None,
            }
            .into(),
        ),

        ..Default::default()
    };

    let msg = whatsapp::Message {
        interactive_message: MessageField::some(outer),
        ..Default::default()
    };

    ctx.send().raw(msg).await?;

    ctx.send().text("success").await
}
