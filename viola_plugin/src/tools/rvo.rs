use viola_core::context::{Context, media::MediaRef};
use viola_macros::command;

#[command(trigger = ["rvo", "read", "show", "view"])]
async fn rvo(ctx: Context) -> anyhow::Result<()> {
    if let Ok(media) = ctx.media().quoted() {
        match media {
            MediaRef::Image(img) => {
                let download = ctx.media().download(img).await?;
                ctx.send().image(download).await?;
                Ok(())
            }
            MediaRef::Video(vid) => {
                let download = ctx.media().download(vid).await?;
                ctx.send().video(download).await?;
                Ok(())
            }
            MediaRef::Audio(aud) => {
                let download = ctx.media().download(aud).await?;
                ctx.send().audio(download).await?;
                Ok(())
            }
            _ => ctx.send().reply_failed().await,
        }
    } else {
        ctx.send().reply_failed().await
    }
}
