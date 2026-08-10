mod bot;
mod cli;
mod client;
mod handler;
mod incoming;
mod parser;
mod store;

use ahash::AHashMap;
use std::{io::Write, process::ExitCode, sync::LazyLock};
use viola_command as _;
use viola_core::{COMMANDS, Command};

pub static COMMAND_MAP: LazyLock<AHashMap<&'static str, &'static Command>> = LazyLock::new(|| {
    let mut map = AHashMap::new();
    for cmd in COMMANDS {
        for t in cmd.triggers {
            map.insert(*t, cmd);
        }
    }
    map.shrink_to_fit();
    map
});

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Always)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{:<5}] [{}] - {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();

    let mut args = std::env::args().skip(1);

    let cmd = match cli::CliCommand::parse(&mut args) {
        Ok(cmd) => cmd,
        Err(err) => {
            log::error!("{err}");
            println!("{}", cli::HELP);
            return ExitCode::FAILURE;
        }
    };

    match cmd.execute().await {
        Ok(()) => ExitCode::SUCCESS,

        Err(err) => {
            log::error!("{err}");
            ExitCode::FAILURE
        }
    }
}
