use std::{
    process,
    sync::{LazyLock, Mutex},
};

use humansize::{DECIMAL, format_size};
use linkme::distributed_slice;
use sysinfo::{ProcessesToUpdate, System};
use viola_core::{COMMANDS, Command, Context};
use whatsapp_rust::{anyhow, chrono::Utc};

#[distributed_slice(COMMANDS)]
static CMD: Command = Command {
    name: "ping",
    triggers: &["ping", "p"],
    category: "tools",
    owner_only: false,
    group_only: false,
    help: None,
    description: None,
    execute: |ctx: Context| Box::pin(execute(ctx)),
};

static SYSTEM: LazyLock<Mutex<System>> = LazyLock::new(|| Mutex::new(System::new_all()));

async fn execute(ctx: Context) -> anyhow::Result<()> {
    let (bot_ram, bot_cpu, total_ram, used_ram, platform) = {
        let mut system = SYSTEM.lock().unwrap();

        let pid = sysinfo::Pid::from_u32(process::id());

        system.refresh_memory();
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

        let process = system.process(pid);

        (
            process.map(|p| p.memory()).unwrap_or_default(),
            process.map(|p| p.cpu_usage()).unwrap_or_default(),
            system.total_memory(),
            system.used_memory(),
            System::name().unwrap_or_else(|| "Unknown".to_string()),
        )
    };

    let latency = Utc::now() - ctx.info.timestamp;

    let text = format!(
        concat!(
            "*Latency:*\n{:.3} ms\n\n",
            "*CPU Usage:*\n{:.2}%\n\n",
            "*System RAM:*\n{} / {}\n\n",
            "*Bot RAM:*\n{}\n\n",
            "*Platform:*\n{}",
            // "*Uptime:*\n{}"
        ),
        latency.num_milliseconds(),
        bot_cpu,
        format_size(used_ram, DECIMAL),
        format_size(total_ram, DECIMAL),
        format_size(bot_ram, DECIMAL),
        platform,
        // uptime
    );

    ctx.send()
        .interactive()
        .inapp_signup(text)
        .title("Pong!")
        .quoted()
        .await
}
