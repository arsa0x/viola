// use crate::framework::context::Context;
// use anyhow::Result;
// use macros::plugin;

// #[plugin( trigger = ["ping","p"], owner = true, plugin_type = "tools" )]
// async fn ping(ctx: Context) -> Result<()> {
//     ctx.reply("pong").await?;

//     Ok(())
// }
use macros::command;

use crate::framework::context::Context;

#[command(["ping", "p"])]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("pong\n{} ms", ctx.elapsed_ms())).await?;
    Ok(())
}
