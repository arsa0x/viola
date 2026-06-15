use viola_core::context::Context;
use viola_macros::command;

#[command(trigger = ["rvo"])]
async fn rvo(ctx: Context) -> anyhow::Result<()> {
    if let Ok((media, media_type)) = ctx.get_media() {
        let download = ctx.msg_ctx.client.download(media).await?;

        let msg = ctx.media_message(download, media_type, None).await?;
        ctx.reply(msg).await?;
        Ok(ctx.reply_success().await?)
    } else {
        Ok(ctx.reply_failed().await?)
    }
}
