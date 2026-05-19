use anyhow::anyhow;
use viola_core::context::Context;
use viola_macros::command;

#[command(trigger = ["d"], owner = true)]
async fn debug(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("{:?}", ctx.msg.message)).await?;
    ctx.reply(&format!("{:?}", ctx.msg.info)).await?;
    ctx.reply(&format!("{:?}", ctx.msg.message_key())).await?;
    ctx.reply(&format!(
        "{:?}",
        ctx.msg
            .client
            .get_lid()
            .await
            .ok_or_else(|| anyhow!("error"))?
    ))
    .await?;
    Ok(())
}
