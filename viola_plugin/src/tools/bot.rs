use viola_core::{config::BotMode, context::Context};
use viola_macros::command;

#[command(triggers = ["bot"], owner = true, category = "tools")]
async fn bot(ctx: Context) -> anyhow::Result<()> {
    let cmd = ctx.args[0].as_str();
    match cmd {
        "owner" => ctx.state.set_bot_mode(BotMode::Owner).await?,
        "group" => ctx.state.set_bot_mode(BotMode::Group).await?,
        "public" => ctx.state.set_bot_mode(BotMode::Public).await?,
        "disabled" => ctx.state.set_bot_mode(BotMode::Disabled).await?,
        _ => {
            ctx.send().failed().await?;
            return Ok(());
        }
    }
    ctx.send().success().await
}
