use std::{
    process,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use humansize::{DECIMAL, format_size};
use sysinfo::{ProcessesToUpdate, System};
use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::anyhow;

static SYSTEM: LazyLock<Mutex<System>> = LazyLock::new(|| Mutex::new(System::new_all()));

#[command(
    triggers = ["status", "stats", "system", "sys"],
    category = "tools",
    description = "Show bot and system status",
)]
async fn status(ctx: Context) -> anyhow::Result<()> {
    let metrics = collect_metrics()?;

    let text = format!(
        "> *Bot*\n\
         CPU: {:.2}%\n\
         RAM: {}\n\n\
         > *System*\n\
         RAM: {} / {}\n\
         CPUs: {}\n\n\
         > *Runtime*\n\
         Uptime: {}\n\
         Platform: {}\n\
         Architecture: {}",
        metrics.bot_cpu,
        format_size(metrics.bot_ram, DECIMAL),
        format_size(metrics.used_ram, DECIMAL),
        format_size(metrics.total_ram, DECIMAL),
        metrics.cpu_count,
        format_uptime(metrics.uptime),
        metrics.platform,
        std::env::consts::ARCH,
    );

    ctx.send()
        .inapp_signup(text)
        .title("Status")
        .quoted()
        .await?;

    Ok(())
}

fn collect_metrics() -> anyhow::Result<SystemMetrics> {
    let mut system = SYSTEM
        .lock()
        .map_err(|_| anyhow::anyhow!("system metrics mutex poisoned"))?;

    let pid = sysinfo::Pid::from_u32(process::id());

    system.refresh_memory();

    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    let process = system.process(pid);

    let bot_ram = process.map(|p| p.memory()).unwrap_or_default();

    let bot_cpu = process.map(|p| p.cpu_usage()).unwrap_or_default();

    let uptime = process.map(|p| p.run_time()).unwrap_or_default();

    Ok(SystemMetrics {
        bot_ram,
        bot_cpu,
        total_ram: system.total_memory(),
        used_ram: system.used_memory(),
        cpu_count: system.cpus().len(),
        uptime,
        platform: System::name().unwrap_or_else(|| "Unknown".into()),
    })
}

fn format_uptime(seconds: u64) -> String {
    let duration = Duration::from_secs(seconds);

    let days = duration.as_secs() / 86_400;
    let hours = (duration.as_secs() % 86_400) / 3_600;
    let minutes = (duration.as_secs() % 3_600) / 60;
    let seconds = duration.as_secs() % 60;

    if days > 0 {
        return format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds);
    }

    if hours > 0 {
        return format!("{}h {:02}m {:02}s", hours, minutes, seconds);
    }

    if minutes > 0 {
        return format!("{}m {:02}s", minutes, seconds);
    }

    format!("{}s", seconds)
}

struct SystemMetrics {
    bot_ram: u64,
    bot_cpu: f32,
    total_ram: u64,
    used_ram: u64,
    cpu_count: usize,
    uptime: u64,
    platform: String,
}
