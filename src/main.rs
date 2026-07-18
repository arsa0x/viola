mod bot;
mod client;
mod handler;
mod incoming;
mod parser;
mod store;

use std::{io::Write, process::ExitCode, sync::LazyLock};

use ahash::AHashMap;
use viola_command as _;
use viola_core::{COMMANDS, Command, config};

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

fn main() -> ExitCode {
    init_logger();

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("init") => match config::init_project() {
            Ok(path) => {
                let name = config::binary_name();
                log::info!("project created at ./{}", path.display());
                log::info!("next steps:");
                log::info!("  cd {}", path.display());
                log::info!("  edit the `config` file to your liking");
                log::info!("  ./{name}   (run this from inside that folder to start the bot)");
                ExitCode::SUCCESS
            }
            Err(err) => {
                log::error!("failed to create project: {err}");
                ExitCode::FAILURE
            }
        },
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            log::error!("unknown command: {other}");
            print_help();
            ExitCode::FAILURE
        }
        None => run(),
    }
}

fn run() -> ExitCode {
    // Forces validation + config load now. If this isn't a valid project
    // directory, the process exits with a clear error before the tokio
    // runtime is even built — the bot will NOT run outside a project folder.
    LazyLock::force(&config::CONFIG);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(bot::run());
    ExitCode::SUCCESS
}

fn init_logger() {
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
}

fn print_help() {
    let name = config::binary_name();
    eprintln!(
        "USAGE:\n  {name} init     create a new project in ./{name}\n  {name}          run the bot (must be launched from inside a {name} project directory)\n"
    );
}
