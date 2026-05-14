# Viola WhatsApp Bot

Fast and modular WhatsApp bot built with Rust and [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) library.

## Features

- Fast async architecture with Tokio
- Auto command module discovery
- Low memory usage
- Modular command architecture
- Procedural macro commands
- Inventory-based auto registration

## Installation

```sh
git clone https://github.com/yourusername/viola.git
cd viola

cargo run
```

## Example Command

```rs
use macros::command;

use crate::framework::context::Context;

#[command(["ping", "p"])]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}
```

## Project Structure

```sh
.
├── src
│   ├── commands # all commands/plugins in here
│   ├── framework
│   ├── macros
│   │   └── src
│   ├── middlewares
│   └── utils
└── store
```

## License

[MIT](https://github.com/arsa0x/viola/blob/main/LICENSE)
