# Viola WhatsApp Bot

A fast and modular WhatsApp bot framework for Rust with procedural macro commands and auto registration. Built with Rust and [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) library.

## Features

- High-performance asynchronous runtime powered by Tokio
- Native Rust performance with a low memory footprint
- Native Rust performance
- Procedural macro command system
- Auto-generated project scaffolding
- Multi-session support (run one or multiple WhatsApp accounts)

## Getting Started

### Installation

```bash
git clone https://github.com/arsa0x/viola.git
cd viola
cargo build --release
```

The compiled binary will be located at:

```bash
target/release/viola
```

You can place the binary anywhere or add it to your `$PATH`.

### Create a session

Create your first WhatsApp session:

```bash
viola session new
```

Or specify a custom name:

```bash
viola session new personal
```

This creates a directory for the session containing its own configuration and authentication data.

### Session Storage

Viola stores all session data in your operating system's standard configuration directory using the [`directories`](https://crates.io/crates/directories) crate.

The layout looks like this:

```text
viola/
└── sessions
    ├── personal
    │   ├── config
    │   └── store.redb
    └── default
        ├── config
        └── store.redb
```

Typical locations are:

| Platform | Location                               |
| -------- | -------------------------------------- |
| Linux    | `~/.config/viola/`                     |
| macOS    | `~/Library/Application Support/viola/` |
| Windows  | `%APPDATA%\viola\`                     |

### Edit the configuration

Each session has its own `config` file.

For example:

```bash
~/.config/viola/
└── sessions
    ├── personal
    │   ├── config
    │   └── store.redb
    └── default
        ├── config
        └── store.redb
```

Edit the configuration before starting the bot.

### Start the bot

Run the only available session:

```bash
viola
```

or

```bash
viola run
```

Run a specific session:

```bash
viola run --session personal
```

Run every session simultaneously:

```bash
viola run --all
```

On the first launch, Viola will display a QR code for pairing with WhatsApp.

## Session Management

Create a session:

```bash
viola session new [name]
```

List all sessions:

```bash
viola session list
```

Remove a session:

```bash
viola session remove <name>
```

## Creating Commands

There are two ways to register a command: the **procedural macro** (recommended for most commands), or **manual registration** via `linkme::distributed_slice` when you need direct control over the `Command` struct.

### Procedural Macro

```rs
use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::anyhow;

#[command(
    triggers = [""],    // required
    category = "",      // required
    owner_only = false, // optional
    group_only = false, // optional
    description = "",   // optional
    help = ""           // optional
)]
async fn command_name(ctx: Context) -> anyhow::Result<()> {
    ctx.send().text("hi there!").quoted().await?;
    Ok(())
}
```

### Manual Registration

```rs
use linkme::distributed_slice;
use viola_core::{COMMANDS, Command, Context};
use whatsapp_rust::anyhow;

#[distributed_slice(COMMANDS)]
static CMD: Command = Command {
    name: "",
    triggers: &[""],
    category: "",
    owner_only: false,
    group_only: false,
    help: None,
    description: None,
    execute: |ctx: Context| Box::pin(execute(ctx)),
};

async fn execute(ctx: Context) -> anyhow::Result<()> {
    ctx.send().text("manual").await
}
```

Both approaches register into the same `COMMANDS` distributed slice, so commands defined either way are discovered and dispatched identically at runtime — pick whichever fits the command better.

## Configuration

Each session maintains its own configuration file.

Example:

```text
# Multiple prefixes separated by |
prefixes=.|!

# WhatsApp owner numbers separated by |
owners=628123456789|628123456780

# Available modes:
# public | group | owner
mode=public
```

## Project Structure

```sh
.
├── src                 # bot entry point
├── viola_core          # command system, context and config
├── viola_macros        # procedural macros for command registration
└── viola_command       # collection of all bot commands
```

## Documentation

- [Message module](./viola_core/src/message/README.md#message-module) — builders for sending text, media, reactions, and interactive (buttons/list) messages via `ctx.send()`.

## License

Licensed under the MIT License.
See [LICENSE](https://github.com/arsa0x/viola/blob/main/LICENSE) for more information.
