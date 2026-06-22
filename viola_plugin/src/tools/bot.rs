use viola_core::context::Context;
use viola_macros::command;

#[command(trigger = ["bot"], owner = true)]
async fn bot(ctx: Context) -> anyhow::Result<()> {
    let cmd = ctx.args[0].as_str();
    let mut config = ctx.state.read_config().await;
    match cmd {
        "on" => {
            config.bot.active = true;
        }
        "off" => {
            config.bot.active = false;
        }
        _ => return Ok(()),
    }
    let _ = ctx.state.save_config(&config);
    ctx.send().reply_success().await
}
