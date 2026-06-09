use viola_core::context::{Context, MediaSource};
use viola_macros::command;

#[command(trigger = ["rvo"])]
async fn rvo(ctx: Context) -> anyhow::Result<()> {
    if ctx.msg.message.extended_text_message.is_some() {
        if ctx.msg.message.view_once_message.is_some()
            || ctx.msg.message.view_once_message_v2.is_some()
            || ctx.msg.message.audio_message.is_some()
        {
            if let Ok((media, media_type)) = ctx.get_media() {
                let download = ctx.client.download(media).await?;

                ctx.reply_media(MediaSource::Bytes(download), media_type, None)
                    .await?;
                ctx.reply_success().await?;
            }
        } else {
            ctx.reply("not a once view message").await?;
        }
    }
    Ok(())
}
