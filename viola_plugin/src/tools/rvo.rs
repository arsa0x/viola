use viola_core::context::{Context, media::MediaRef};
use viola_macros::command;

#[command(
    triggers = ["rvo", "read", "show", "view"],
    category = "tools",
    description = "Read view once message"
)]
async fn read_view_once(ctx: Context) -> anyhow::Result<()> {
    if let Ok(media) = ctx.media().quoted() {
        match media {
            MediaRef::Image(img) => {
                let download = ctx.media().download(img).await?;
                ctx.send().image(download).quoted().await?;
                Ok(())
            }
            MediaRef::Video(vid) => {
                let download = ctx.media().download(vid).await?;
                ctx.send().video(download).quoted().await?;
                Ok(())
            }
            MediaRef::Audio(aud) => {
                let download = ctx.media().download(aud).await?;
                ctx.send().audio(download).quoted().await?;
                Ok(())
            }
            _ => ctx.send().failed().await,
        }
    } else {
        ctx.send().failed().await
    }
}
