use viola_core::context::{Context, MediaSource};
use viola_macros::command;

#[command(trigger = ["rvo"])]
async fn rvo(ctx: Context) -> anyhow::Result<()> {
    if let Ok((media, media_type)) = ctx.get_media() {
        let download = ctx.client.download(media).await?;

        ctx.reply_media(MediaSource::Bytes(download), media_type, None)
            .await?;
        ctx.reply_success().await?;
    }
    Ok(())
}
