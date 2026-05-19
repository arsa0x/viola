use viola_core::context::Context;
use viola_macros::command;

const HELP: &str = r#"USAGE:
  .ping

EXAMPLE:
  .ping
  .p"#;

#[command(trigger = ["ping", "p"], help = HELP, description = "Check bot latency and response time")]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("pong\ntime: {}ms", ctx.elapsed_ms()))
        .await?;
    Ok(())
}
