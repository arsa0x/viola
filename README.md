# Viola

A modular and async WhatsApp bot built with Rust using [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust).

## Features

- Async command handling with Tokio
- Modular command architecture
- Procedural macro commands
- Inventory-based auto registration
- SQLite session storage
- QR code pairing
- Context & router abstraction

---

## Installation

```bash
git clone https://github.com/yourusername/viola.git
cd viola

cargo run
```

Scan the QR code from WhatsApp Linked Devices.

---

## Example Command

```rust
use macros::command;

use crate::framework::context::Context;

#[command(["ping", "p"])]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}
```

Usage:

```text
.ping
```

---

## Project Structure

```text
.
├── src
│   ├── commands
│   ├── framework
│   ├── macros
│   │   └── src
│   ├── middlewares
│   └── utils
└── store
```

## License

[MIT](https://github.com/arsa0x/viola/blob/main/LICENSE)
