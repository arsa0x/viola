use viola_core::{
    Context,
    message::{
        interactive::carousel::{CarouselButton, CarouselCard},
        media::MediaSource,
    },
};
use viola_macros::command;
use whatsapp_rust::anyhow;

#[command(
    triggers = ["t"],
    category = "tools",
    description = "Test Message"
)]
async fn test(ctx: Context) -> anyhow::Result<()> {
    ctx.send()
        .interactive()
        .carousel("Carousel title")
        .footer("Powered by Viola")
        .card(CarouselCard {
            title: "Card A".into(),
            subtitle: None,
            body_text: "Body Card A".into(),
            image: MediaSource::Url("http://127.0.0.1:8000/arona_.png"),
            buttons: vec![CarouselButton::CtaUrl {
                display_text: "My MBG Gw".into(),
                url: "http://127.0.0.1:8000/arona_.png".into(),
                merchant_url: None,
            }],
        })
        .quoted()
        .await?;
    ctx.send().success().await
}
