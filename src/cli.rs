use std::io::{IsTerminal, Write};

use crate::bot;

#[derive(Debug)]
pub enum CliCommand {
    Session(SessionCommand),
    Run(RunCommand),
    Help,
}

#[derive(Debug)]
pub enum SessionCommand {
    List,
    New { name: String },
    Delete { name: String },
}

#[derive(Debug)]
pub enum RunCommand {
    Auto,
    Named { name: String },
    All,
}

pub const HELP: &str = r#"Usage: viola [command] [option]

Commands & Options:
  session new [name]      Create a new session (default: "default")
  session list             List all available sessions
  session delete <name>    Delete a session

run                      Start the bot
  run --session <name>     Start a specific session
  run --all                Start all sessions

General Options:
--help                   Print command usage
"#;

impl CliCommand {
    pub fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, String> {
        match args.next().as_deref() {
            None => Ok(Self::Run(RunCommand::Auto)),

            Some("help" | "--help" | "-h") => Ok(Self::Help),

            Some("session") => Ok(Self::Session(SessionCommand::parse(args)?)),

            Some("run") => Ok(Self::Run(RunCommand::parse(args)?)),

            Some(command) => Err(format!("unknown command: {command}")),
        }
    }

    pub async fn execute(self) -> Result<(), String> {
        match self {
            Self::Help => {
                println!("{HELP}");
                Ok(())
            }

            Self::Session(command) => command.execute(),

            Self::Run(command) => command.execute().await,
        }
    }
}
impl SessionCommand {
    pub fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, String> {
        match args.next().as_deref() {
            Some("list") => Ok(Self::List),

            Some("new") => {
                let name = args.next().unwrap_or_else(|| "default".to_owned());

                Ok(Self::New { name })
            }

            Some("delete") => {
                let name = args
                    .next()
                    .ok_or_else(|| "usage: viola session delete <name>".to_owned())?;

                Ok(Self::Delete { name })
            }

            Some(command) => Err(format!("unknown session subcommand: {command}")),

            None => Err("missing session subcommand. \
                     Use `list`, `new`, or `delete`."
                .to_owned()),
        }
    }

    pub fn execute(self) -> Result<(), String> {
        match self {
            Self::New { name } => Self::create_session(&name),
            Self::List => Self::list_sessions(),
            Self::Delete { name } => Self::delete_session(&name),
        }
    }

    fn list_sessions() -> Result<(), String> {
        let sessions = viola_core::session::list_sessions()
            .map_err(|err| format!("failed to read session directory: {err}"))?;

        if sessions.is_empty() {
            println!(
                "No sessions found. \
                 Run `viola session new <name>` to create one."
            );

            return Ok(());
        }

        println!("Available sessions:");

        for name in sessions {
            println!("  - {name}");
        }

        Ok(())
    }

    fn create_session(name: &str) -> Result<(), String> {
        let sessions = viola_core::session::list_sessions().map_err(|err| err.to_string())?;

        if sessions.iter().any(|session| session == name) {
            return Err(format!("session '{name}' already exists"));
        }

        let dir = viola_core::session::ensure_session_dir(name)
            .map_err(|err| format!("failed to create session directory: {err}"))?;

        viola_core::config::ensure_config_file(&dir)
            .map_err(|err| format!("failed to create configuration file: {err}"))?;

        println!("Session '{name}' created.");
        println!("Configuration: {}", dir.join("config").display());

        Ok(())
    }

    fn delete_session(name: &str) -> Result<(), String> {
        viola_core::session::remove_session(name)
            .map_err(|err| format!("failed to delete session '{name}': {err}"))?;

        println!("Session '{name}' deleted.");

        Ok(())
    }
}

impl RunCommand {
    pub fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, String> {
        match args.next().as_deref() {
            None => Ok(Self::Auto),

            Some("--all") => Ok(Self::All),

            Some("--session") => {
                let name = args.next().ok_or_else(|| {
                    "--session requires a session name, \
                     for example: --session wa_bot_1"
                        .to_owned()
                })?;

                Ok(Self::Named { name })
            }

            Some(option) => Err(format!("unknown run option: {option}")),
        }
    }

    pub async fn execute(self) -> Result<(), String> {
        match self {
            Self::Auto => Self::run_auto().await,
            Self::Named { name } => Self::run_named(&name).await,
            Self::All => Self::run_all().await,
        }
    }

    fn prompt_session_selection(sessions: &[String]) -> Option<Vec<String>> {
        if !std::io::stdin().is_terminal() {
            log::warn!(
                "stdin is not attached to a TTY; \
                 automatically starting all {} sessions",
                sessions.len()
            );

            return Some(sessions.to_vec());
        }

        println!("Found {} sessions:", sessions.len());

        for (index, name) in sessions.iter().enumerate() {
            println!("  [{}] {}", index + 1, name);
        }

        println!("  [a] Start all sessions");

        print!("Select: ");
        std::io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok()?;

        let input = input.trim();

        if input.eq_ignore_ascii_case("a") {
            return Some(sessions.to_vec());
        }

        match input.parse::<usize>() {
            Ok(index) if (1..=sessions.len()).contains(&index) => {
                Some(vec![sessions[index - 1].clone()])
            }

            _ => {
                log::error!("invalid selection: {input}");
                None
            }
        }
    }

    async fn run_auto() -> Result<(), String> {
        let sessions = viola_core::session::list_sessions().map_err(|err| err.to_string())?;

        match sessions.len() {
            0 => Err("no sessions found. \
                 Run `viola session new <name>` first."
                .into()),

            1 => {
                bot::run_sessions(sessions).await;
                Ok(())
            }

            _ => {
                let sessions = Self::prompt_session_selection(&sessions)
                    .ok_or_else(|| "no session selected".to_owned())?;

                bot::run_sessions(sessions).await;

                Ok(())
            }
        }
    }

    async fn run_named(name: &str) -> Result<(), String> {
        let sessions = viola_core::session::list_sessions().map_err(|err| err.to_string())?;

        if !sessions.iter().any(|session| session == name) {
            return Err(format!(
                "session '{name}' not found. \
                 Run `viola session new {name}` first."
            ));
        }

        bot::run_sessions(vec![name.to_owned()]).await;

        Ok(())
    }

    async fn run_all() -> Result<(), String> {
        let sessions = viola_core::session::list_sessions().map_err(|err| err.to_string())?;

        if sessions.is_empty() {
            return Err("no sessions found. \
                 Run `viola session new <name>` first."
                .into());
        }

        bot::run_sessions(sessions).await;

        Ok(())
    }
}
