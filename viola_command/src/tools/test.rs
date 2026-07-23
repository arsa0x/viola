use viola_core::{Context, message::media::MediaSource};
use viola_macros::command;
use whatsapp_rust::anyhow;

#[command(
    triggers = ["t"],
    category = "tools",
    description = "Test Message"
)]
async fn test(ctx: Context) -> anyhow::Result<()> {
    ctx.send()
        .image(MediaSource::Url("http://127.0.0.1:8000/arona_.png"))
        .quoted()
        .await
}
