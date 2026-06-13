use anyhow::anyhow;
use viola_core::context::Context;
use viola_macros::command;

#[command(trigger = ["d"], owner = true)]
async fn debug(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("message: \n\n```{:#?}```", ctx.msg.message))
        .await?;
    ctx.reply(&format!(
        "message_key: \n\n```{:#?}```",
        ctx.msg.message_key()
    ))
    .await?;
    ctx.reply(&format!("info: \n\n```{:#?}```", ctx.msg.info))
        .await?;
    ctx.reply(&format!(
        "lid: \n\n```{:#?}```",
        ctx.msg.client.get_lid().ok_or_else(|| anyhow!("error"))?
    ))
    .await?;
    Ok(())
}
