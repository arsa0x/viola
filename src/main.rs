mod bot;
mod client;
mod debug;
mod handler;
mod incoming;
mod parser;
mod store;

use ahash::AHashMap;
use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
    sync::LazyLock,
};
use viola_command as _;
use viola_core::{COMMANDS, Command, config, session};

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

enum RunSelector {
    Auto,
    Named(String),
    All,
}

fn main() -> ExitCode {
    init_logger();
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        Some("session") => match args.next().as_deref() {
            Some("new") => cmd_session_new(&args.next().unwrap_or_else(|| "default".into())),
            Some("list") => cmd_session_list(),
            Some("remove") => match args.next() {
                Some(name) => cmd_session_remove(&name),
                None => {
                    log::error!("usage: viola session remove <name>");
                    ExitCode::FAILURE
                }
            },
            other => {
                log::error!("unknown session subcommand: {other:?}");
                print_help();
                ExitCode::FAILURE
            }
        },
        Some("run") => run(parse_run_selector(args)),
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            log::error!("unknown command: {other}");
            print_help();
            ExitCode::FAILURE
        }
        None => run(RunSelector::Auto),
    }
}

fn parse_run_selector(mut args: impl Iterator<Item = String>) -> RunSelector {
    match args.next().as_deref() {
        Some("--all") => RunSelector::All,
        Some("--session") => match args.next() {
            Some(name) => RunSelector::Named(name),
            None => {
                log::error!("--session requires a session name, for example: --session wa_bot_1");
                std::process::exit(1)
            }
        },
        _ => RunSelector::Auto,
    }
}

fn cmd_session_new(name: &str) -> ExitCode {
    let existing = session::list_sessions().unwrap_or_default();
    if existing.iter().any(|s| s == name) {
        log::error!("session '{name}' already exists");
        return ExitCode::FAILURE;
    }
    let dir = match session::ensure_session_dir(name) {
        Ok(d) => d,
        Err(err) => {
            log::error!("failed to create session directory: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = config::ensure_config_file(&dir) {
        log::error!("failed to create configuration file: {err}");
        return ExitCode::FAILURE;
    }
    log::info!("session '{name}' created at {}", dir.display());
    log::info!("Edit the configuration file if needed, then run:");
    log::info!(
        "  viola run --session {name}   (a QR code will be shown automatically the first time)"
    );
    ExitCode::SUCCESS
}

fn cmd_session_list() -> ExitCode {
    match session::list_sessions() {
        Ok(s) if s.is_empty() => {
            println!("No sessions found. Run `viola session new <name>` to create one.");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            println!("Available sessions:");
            for n in s {
                println!("  - {n}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            log::error!("failed to read session directory: {err}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_session_remove(name: &str) -> ExitCode {
    let dir = session::session_path(name);
    if !dir.exists() {
        log::error!("session '{name}' not found");
        return ExitCode::FAILURE;
    }
    match std::fs::remove_dir_all(&dir) {
        Ok(_) => {
            log::info!("session '{name}' removed");
            ExitCode::SUCCESS
        }
        Err(err) => {
            log::error!("failed to remove session: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(selector: RunSelector) -> ExitCode {
    let sessions = match session::list_sessions() {
        Ok(s) => s,
        Err(err) => {
            log::error!("failed to read session directory: {err}");
            return ExitCode::FAILURE;
        }
    };

    let to_run = match selector {
        RunSelector::All => sessions,
        RunSelector::Named(name) => {
            if !sessions.contains(&name) {
                log::error!("session '{name}' not found. Run `viola session new {name}` first.");
                return ExitCode::FAILURE;
            }
            vec![name]
        }
        RunSelector::Auto => match sessions.len() {
            0 => {
                log::error!("no sessions found. Run `viola session new` first.");
                return ExitCode::FAILURE;
            }
            1 => sessions,
            _ => match prompt_session_selection(&sessions) {
                Some(s) => s,
                None => return ExitCode::FAILURE,
            },
        },
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(bot::run_sessions(to_run));
    ExitCode::SUCCESS
}

fn prompt_session_selection(sessions: &[String]) -> Option<Vec<String>> {
    if !io::stdin().is_terminal() {
        log::warn!(
            "stdin is not attached to a TTY; automatically starting all {} sessions",
            sessions.len()
        );
        return Some(sessions.to_vec());
    }

    println!("Found {} sessions:", sessions.len());
    for (i, name) in sessions.iter().enumerate() {
        println!("  [{}] {}", i + 1, name);
    }
    println!("  [a] Start all sessions");
    print!("Select: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let input = input.trim();

    if input.eq_ignore_ascii_case("a") {
        return Some(sessions.to_vec());
    }
    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= sessions.len() => Some(vec![sessions[n - 1].clone()]),
        _ => {
            log::error!("invalid selection: {input}");
            None
        }
    }
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
    eprintln!(
        r#"Usage: viola [command] [option]

Commands & Options:
  session new [name]      Create a new session (default: "default")
  session list            List all available sessions
  session remove <name>   Remove a session

  run                     Start the bot (starts directly if only one session exists)
  run --session <name>    Start a specific session
  run --all               Start all sessions

General Options:
  --help                  Print command usage
      "#
    );
}
