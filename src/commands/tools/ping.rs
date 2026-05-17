use macros::command;

use crate::framework::context::Context;

const HELP: &str = r#"
Check bot latency and response time.

USAGE:
  .ping

ALIASES:
  .p

EXAMPLES:
  .ping
  .p
"#;

#[command(trigger = ["ping", "p"], help = HELP)]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply(&format!("pong\ntime: {}ms", ctx.elapsed_ms()))
        .await?;
    Ok(())
}
