# Viola WhatsApp Bot

A fast and modular WhatsApp bot framework for Rust with procedural macro commands and inventory-based auto registration. Built with Rust and [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) library.

## Features

- Tokio async runtime
- Inventory-based automatic command registration
- Procedural macro command system
- Lua plugin support
- Low memory footprint
- Native Rust performance
- Modular plugin architecture
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

## Native Command Example

```rs
use viola_core::framework::context::Context;
use viola_macros::command;

#[command(
    trigger = [""],
    owner = false,
    group_only = false,
    description = "",
    help = ""
)]
async fn name(ctx: Context) -> anyhow::Result<()> {
    // do something

    Ok(())
}
```

### Example

```rs
use viola_core::framework::context::Context;
use viola_macros::command;

const HELP: &str = r#"USAGE:
.ping

EXAMPLE:
.ping
.p"#;

#[command(
    trigger = ["ping", "p"],
    description = "Ping command",
    help = HELP
)]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply("pong").await?;

    Ok(())
}
```

## Lua Plugin Example

```lua
return {
    triggers = { "ping", "p" },
    description = "Ping command",

    exec = function(ctx)
        ctx:reply("pong from lua!")
    end
}
```

Lua plugins are automatically loaded from:

```txt
$data_dirs/viola/plugins/
```

## Configuration

Viola automatically stores configuration files inside your system "data_dir" using the "dirs" crate.

### Config Location

| OS      | Path                                                      |
|---------|-----------------------------------------------------------|
| Linux   | `~/.local/share/viola/config.toml`                        |
| Windows | `C:\Users\<user>\AppData\Roaming\viola\config.toml`       |
| macOS   | `~/Library/Application Support/viola/config.toml`         |

The config file will be automatically generated on first run.

### Example Config

```toml
[bot]
name = "viola"
prefix = "."
owner = "628123456789"
```

## Project Structure

```sh
.
├── src                 # bot entry point
├── viola_core          # command system, router, context, config, and state
├── viola_macros        # procedural macros for command registration
└── viola_plugin        # collection of all bot native and lua plugins
    ├── lua             # will be moved to $data_dir/viola/plugins/
    │   ├── downloader
    │   └── tools
    └── src
        ├── downloader
        └── tools
```

## License
Licensed under the MIT License.
See [LICENSE](https://github.com/arsa0x/viola/blob/main/LICENSE) for more information.
