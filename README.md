# Viola WhatsApp Bot

A fast and modular WhatsApp bot framework for Rust with procedural macro commands and auto registration. Built with Rust and [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) library.

## Features

- Tokio async runtime
- Procedural macro command system
- Low memory footprint
- Native Rust performance
- Auto-generated configuration

## Getting Started

```bash
git clone https://github.com/arsa0x/viola.git
cd viola

cargo run
```

On first launch, Viola will:

- generate configuration files
- create storage directories
- display a pairing QR code

## Example

```rs
use viola_core::context::Context;
use viola_macros::command;

#[command(
    trigger = [""],     // required
    category = "",      // required
    owner = false,      // optional
    group_only = false, // optional
    description = "",   // optional
    help = ""           // optional
)]
async fn command_name(ctx: Context) -> anyhow::Result<()> {
    ctx.send().text("hi there!").quoted().await?;
    Ok(())
}
```

## Configuration

Viola automatically stores configuration files inside your system home directory.

### Config Location

|Platform | Example Path                        |
| ------- | ----------------------------------- |
| Linux   | /home/username/viola/config.toml    |
| macOS   | /Users/UserName/viola/config.toml   |
| Windows | C:\Users\UserName\viola\config.toml |

The config file will be automatically generated on first run.

### Example Config

```toml
[bot]
name   = "viola"
prefix = "."
owner  = "628123456789"
mode   = "public"
```

## Project Structure

```sh
.
├── src                 # bot entry point
├── viola_core          # command system, router, context, config, and state
├── viola_macros        # procedural macros for command registration
└── viola_plugin        # collection of all bot plugins
    └── src
        ├── downloader
        └── tools
```

## License

Licensed under the MIT License.
See [LICENSE](https://github.com/arsa0x/viola/blob/main/LICENSE) for more information.
