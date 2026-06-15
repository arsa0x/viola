use anyhow::anyhow;
use viola_core::context::Context;
use viola_macros::command;

#[command(trigger = ["d"], owner = true)]
async fn debug(ctx: Context) -> anyhow::Result<()> {
    ctx.reply_text(&format!("message: \n\n```{:#?}```", ctx.msg_ctx.message))
        .await?;
    ctx.reply_text(&format!(
        "message_key: \n\n```{:#?}```",
        ctx.msg_ctx.message_key()
    ))
    .await?;
    ctx.reply_text(&format!("info: \n\n```{:#?}```", ctx.msg_ctx.info))
        .await?;
    ctx.reply_text(&format!(
        "lid: \n\n```{:#?}```",
        ctx.msg_ctx
            .client
            .get_lid()
            .ok_or_else(|| anyhow!("error"))?
    ))
    .await?;
    Ok(())
}
