use macros::command;

use crate::framework::context::Context;

#[command(trigger = ["ping", "p"])]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("pong\ntime: {}ms", ctx.elapsed_ms()))
        .await?;
    Ok(())
}
