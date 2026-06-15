use humansize::{DECIMAL, format_size};
use std::process;
use sysinfo::{ProcessesToUpdate, System};
use viola_core::context::Context;
use viola_macros::command;

fn format_duration(duration: std::time::Duration) -> String {
    let days = duration.as_secs() / 86400;
    let hours = (duration.as_secs() % 86400) / 3600;
    let mins = (duration.as_secs() % 3600) / 60;

    format!("{days}d {hours}h {mins}m")
}

const HELP: &str = r#"USAGE:
  .ping

EXAMPLE:
  .ping
  .p"#;

#[command(
    trigger = ["ping", "p"],
    help = HELP,
    description = "Check bot latency and response time"
)]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    let mut system = System::new_all();

    system.refresh_processes(ProcessesToUpdate::All, true);

    let pid = sysinfo::Pid::from_u32(process::id());

    let process = system.process(pid);

    let bot_ram = process.map(|p| p.memory()).unwrap_or_default();

    let bot_cpu = process.map(|p| p.cpu_usage()).unwrap_or_default();

    let total_ram = system.total_memory();
    let used_ram = system.used_memory();

    let platform = System::name().unwrap_or_else(|| "Unknown".to_string());

    let uptime = format_duration(ctx.state.start_time.elapsed());

    let processing = ctx.processing_ms();

    let text = format!(
        concat!(
            "*Pong!*\n\n",
            "*Processing    :* {:.3} ms\n",
            "*CPU Usage     :* {:.2}%\n",
            "*System RAM    :* {} / {}\n",
            "*Bot RAM       :* {}\n",
            "*Platform      :* {}\n",
            "*Uptime        :* {}\n"
        ),
        processing,
        bot_cpu,
        format_size(used_ram, DECIMAL),
        format_size(total_ram, DECIMAL),
        format_size(bot_ram, DECIMAL),
        platform,
        uptime
    );
    ctx.reply(&text).await?;

    Ok(())
}
