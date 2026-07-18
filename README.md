# Viola WhatsApp Bot

A fast and modular WhatsApp bot framework for Rust with procedural macro commands and auto registration. Built with Rust and [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) library.

## Features

- Tokio async runtime
- Procedural macro command system
- Low memory footprint
- Native Rust performance
- Auto-generated project scaffolding

## Getting Started

```bash
git clone https://github.com/arsa0x/viola.git
cd viola
cargo build --release
```

Then, from wherever you want your bot's data to live:

```bash
/path/to/viola/target/release/viola init
cd viola
../path/to/viola/target/release/viola
```

- `viola init` creates a new `viola/` project directory containing the config file, a `download/` folder, and a `cache/` folder.
- Running `viola` **must be done from inside that generated folder** — it looks for `config`, `download/`, and `cache/` in the current directory and refuses to start if they're missing.
- On first run inside the project folder, it will display a pairing QR code to link your WhatsApp account.

## Example

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

See [`viola_core/src/message/README.md`](./viola_core/src/message/README.md#message-module) for the full API used to build and send messages (text, media, reactions, interactive buttons/lists) via `ctx.send()`.

## Configuration

Each project folder created by `viola init` has its own `config` file — there is no global config in your home directory. Multiple projects (e.g. for multiple bot accounts) can live side by side, each with its own folder.

### Config Location

The config file is simply `config`, sitting next to `download/` and `cache/` inside the project directory you `cd` into before running the bot:

```
viola/
├── config
├── download/
└── cache/
```

### Config Format

The config is a plain `key=value` file — one setting per line, `#` for comments:

```
# prefixes accepts multiple single-character prefixes, separated by |
prefixes=.|!

# owners accepts a list of WhatsApp numbers, separated by |
owners=628123456789|628123456780

# mode must be one of: public, group, owner
mode=public
```

## Project Structure

```sh
.
├── src                 # bot entry point (CLI: init / run)
├── viola_core          # command system, context and config
├── viola_macros        # procedural macros for command registration
└── viola_command       # collection of all bot commands
```

## Documentation

- [Message module](./viola_core/src/message/README.md#message-module) — builders for sending text, media, reactions, and interactive (buttons/list) messages via `ctx.send()`.

## License

Licensed under the MIT License.
See [LICENSE](https://github.com/arsa0x/viola/blob/main/LICENSE) for more information.
