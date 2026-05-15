use anyhow::anyhow;
use macros::command;

use crate::framework::context::Context;

#[command(trigger = ["t"], owner = true)]
async fn _test(ctx: Context) -> anyhow::Result<()> {
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
