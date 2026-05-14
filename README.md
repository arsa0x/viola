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

```sh
git clone https://github.com/yourusername/viola.git
cd viola

cargo run
```

Scan the QR code from WhatsApp Linked Devices.

---

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

Usage:

```txt
.ping
```

---

## Project Structure

```sh
.
├── Cargo.lock
├── Cargo.toml
├── LICENSE
├── README.md
├── src
│   ├── commands # all commands/plugins in here
│   │   ├── mod.rs
│   │   └── ping.rs
│   ├── framework
│   │   ├── command.rs
│   │   ├── context.rs
│   │   ├── mod.rs
│   │   ├── router.rs
│   │   └── state.rs
│   ├── lib.rs
│   ├── macros
│   │   ├── Cargo.lock
│   │   ├── Cargo.toml
│   │   └── src
│   │       └── lib.rs
│   ├── main.rs
│   ├── middlewares
│   │   └── mod.rs
│   └── utils
│       └── mod.rs
└── store
    ├── whatsapp.db
    ├── whatsapp.db-shm
    └── whatsapp.db-wal
```

## License

[MIT](https://github.com/arsa0x/viola/blob/main/LICENSE)
