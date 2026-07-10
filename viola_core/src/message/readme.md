# Message Module

This module provides a fluent, type-safe API for constructing and sending
outbound WhatsApp messages (text, media, reactions, and interactive
messages). It is the "outgoing" counterpart to `Context` — while `Context`
gives you information about an *incoming* message, `message::MessageFactory`
(accessed via `ctx.send()`) lets you build and dispatch a *new* one.

Every concrete message type follows the same **builder pattern**:

1. Start from `ctx.send()`, which returns a [`MessageFactory`](#messagefactory).
2. Call the constructor for the message type you want (`.text(...)`,
   `.image(...)`, `.reaction(...)`, etc.). This returns a type-specific
   builder (e.g. `TextBuilder`, `ImageBuilder`).
3. Chain optional setters on that builder (`.quoted()`, `.caption(...)`,
   `.footer(...)`, ...).
4. Either call `.send().await` explicitly, or simply `.await` the builder
   directly — every builder implements `IntoFuture`, so `builder.await` is
   equivalent to `builder.send().await`.

```rust
// explicit .send()
ctx.send().text("pong").send().await?;

// equivalent, via IntoFuture
ctx.send().text("pong").await?;

// with a modifier
ctx.send().text("pong").quoted().await?;
```

Because every builder implements `IntoFuture` and borrows `Context` for its
lifetime (`'a`), you always `.await` inside the same scope as `ctx`.

---

## `MessageFactory`

`MessageFactory<'a>` is the entry point, returned by `ctx.send()`. It exposes
one constructor per message type, plus a low-level `raw()` escape hatch and a
few reaction shortcuts.

| Method | Returns | Purpose |
|---|---|---|
| `raw(whatsapp::Message)` | `anyhow::Result<()>` (awaited immediately) | Send a fully custom `whatsapp::Message` protobuf, bypassing all builders. Every builder ultimately calls this internally. |
| `text(impl Into<String>)` | `TextBuilder` | Plain or quoted text message. |
| `image(Vec<u8>)` | `ImageBuilder` | Image message (uploads bytes first). |
| `video(Vec<u8>)` | `VideoBuilder` | Video message (uploads bytes first). |
| `audio(Vec<u8>)` | `AudioBuilder` | Audio message, optionally a voice note (PTT). |
| `document(Vec<u8>)` | `DocumentBuilder` | Document/file attachment. |
| `sticker(Vec<u8>)` | `StickerBuilder` | Sticker message. |
| `interactive()` | `InteractiveFactory` | Entry point for buttons/list messages (see [Interactive messages](#interactive-messages)). |
| `reaction(&str)` | `ReactionBuilder` | React to the *current inbound message* with an emoji. |
| `success()` | `anyhow::Result<()>` (awaited immediately) | Shortcut for `.reaction("✅")`. |
| `wait()` | `anyhow::Result<()>` (awaited immediately) | Shortcut for `.reaction("⏳")`. |
| `failed()` | `anyhow::Result<()>` (awaited immediately) | Shortcut for `.reaction("❌")`. |

`raw()` sends via `ctx.msg_ctx.client.send_message(chat_jid, message)` — this
is the single choke point all outbound messages pass through, so any
cross-cutting concern (logging, rate limiting, retries) belongs there.

---

## Common builder conventions

All media/text/interactive builders share these conventions:

- **`.quoted()`** — marks the outgoing message as a reply to the message
  currently being handled (`ctx.info().ctx_info()`), by attaching
  `context_info`. Without it, the message is sent standalone.
- **Builder fields are private-by-construction** — you only ever get a
  builder back from a `MessageFactory`/`InteractiveFactory` method, never
  construct one directly, so required fields are always initialized.
- **`IntoFuture`** — every builder can be awaited directly instead of calling
  `.send()`, which keeps call sites terse (see the example above).
- **Uploads are transparent** — for media types (image/video/audio/document/
  sticker), `.send()` internally calls `ctx.media().upload(bytes, MediaType)`
  before constructing the protobuf message. You just pass raw bytes; you
  never touch upload plumbing directly.

### `TextBuilder`

```rust
ctx.send().text("Hello!").await?;
ctx.send().text("Hello!").quoted().await?;
```

| Setter | Effect |
|---|---|
| `.quoted()` | Reply to the current message. |

Internally: an *unquoted* text is sent as a plain `conversation` field (the
cheapest message shape WhatsApp supports); a *quoted* text is upgraded to an
`extended_text_message` so `context_info` has somewhere to live. This means
quoting a text message is not free — it changes the wire representation —
but this is handled for you automatically.

### `ImageBuilder`

```rust
ctx.send()
    .image(bytes)
    .caption("look at this")
    .thumbnail(thumb_bytes)
    .quoted()
    .await?;
```

| Setter | Effect |
|---|---|
| `.quoted()` | Reply to the current message. |
| `.caption(impl Into<String>)` | Caption shown under the image. |
| `.thumbnail(Vec<u8>)` | JPEG thumbnail bytes (used for the chat-list preview). |

### `VideoBuilder`

Constructed via `ctx.send().video(bytes)`. Based on the fields wired up in
`MessageFactory::video`, it exposes the same shape as `ImageBuilder`:
`.quoted()`, `.caption(...)`, `.thumbnail(...)`.

### `AudioBuilder`

```rust
ctx.send()
    .audio(bytes)
    .ptt()
    .duration(12)
    .mime_type("audio/ogg; codecs=opus")
    .await?;
```

| Setter | Effect |
|---|---|
| `.quoted()` | Reply to the current message. |
| `.ptt()` | Marks the audio as a push-to-talk voice note rather than a regular audio file. |
| `.duration(u32)` | Duration in seconds. |
| `.mime_type(impl Into<String>)` | Overrides the audio MIME type. |

> **Note:** as of this writing, `AudioOptions` is constructed with
> `mimetype`, `duration_seconds`, and `waveform` left at their defaults
> (commented out in `audio.rs`) — only `context_info` is currently wired
> through. If you need duration/mimetype to actually reach WhatsApp, confirm
> `AudioOptions` is being populated from `self.duration` / `self.mime_type`
> before relying on those setters having an effect.

### `DocumentBuilder`

Constructed via `ctx.send().document(bytes)`. Exposed setters (inferred from
its field set): `.quoted()`, `.caption(...)`, `.thumbnail(...)`,
`.mime_type(...)`, `.file_name(...)`, `.title(...)`.

### `StickerBuilder`

Constructed via `ctx.send().sticker(bytes)`. Exposed setters: `.quoted()`,
`.thumbnail(...)`.

## Interactive messages

`ctx.send().interactive()` returns an `InteractiveFactory`, the entry point
for WhatsApp's native "flow" message types — buttons and list pickers. All
four interactive builders wrap the same underlying primitive: a WhatsApp
**Interactive Message** containing a `NativeFlowMessage` with one or more
`NativeFlowButton`s, each carrying a `name` (the flow type) and a
`button_params_json` blob (the flow-specific payload, JSON-encoded).

| Factory method | Returns | Native flow name | Use case |
|---|---|---|---|
| `.raw(interactive_message::InteractiveMessage)` | `InteractiveBuilder` | — | Fully custom interactive payload. |
| `.inapp_signup(text_body)` | `InappSignupBuilder` | `inapp_signup` | WhatsApp's built-in signup flow button. |
| `.quick_reply(Vec<QuickReplyButton>)` | `QuickReplyBuilder` | `quick_reply` | Up to a handful of quick-tap reply buttons. |
| `.single_select(Vec<SingleSelectSection>)` | `SingleSelectBuilder` | `single_select` | A scrollable, sectioned list picker (one selectable option). |
| `.cta_url(Vec<CtaButton>)` | `CtaUrlBuilder` | `cta_url` | Buttons that open an external URL. |

All interactive builders share `.quoted()`, `.title(...)`, `.text_body(...)`,
and (except `inapp_signup`) `.footer(...)`, then send by either calling
`.send().await` or awaiting directly.

### `QuickReplyBuilder`

```rust
ctx.send()
    .interactive()
    .quick_reply(vec![
        QuickReplyButton { text: "Yes".into(), id: "yes".into() },
        QuickReplyButton { text: "No".into(), id: "no".into() },
    ])
    .title("Confirm?")
    .text_body("Do you want to proceed?")
    .footer("Powered by Viola")
    .quoted()
    .await?;
```

### `SingleSelectBuilder`

```rust
ctx.send()
    .interactive()
    .single_select(vec![SingleSelectSection {
        title: "Choose a fruit".into(),
        rows: vec![
            SingleSelectRow { title: "Apple".into(), description: "Red and crisp".into(), id: "apple".into() },
            SingleSelectRow { title: "Banana".into(), description: "Yellow and sweet".into(), id: "banana".into() },
        ],
    }])
    .select_label("View options")
    .text_body("Pick one:")
    .await?;
```

| Setter | Effect |
|---|---|
| `.select_label(impl Into<String>)` | Text on the button that opens the list (defaults to `"Select Options"` if omitted). |

### `CtaUrlBuilder`

```rust
ctx.send()
    .interactive()
    .cta_url(vec![CtaButton {
        display_text: "Visit site".into(),
        id: "visit".into(),
        url: "https://example.com".into(),
        merchant_url: "https://example.com".into(),
    }])
    .title("Check this out")
    .await?;
```

### `InappSignupBuilder`

```rust
ctx.send()
    .interactive()
    .inapp_signup("Sign up in-app")
    .title("Get started")
    .await?;
```

The simplest interactive type — a single fixed button with an empty
`button_params_json` (`"{}"`), no `.footer()` support, and no button list to
configure.

### `InteractiveBuilder` (raw)

For anything the higher-level builders don't cover, build the
`interactive_message::InteractiveMessage` enum yourself and pass it to
`.raw(...)`:

```rust
ctx.send()
    .interactive()
    .raw(my_custom_native_flow_message)
    .body(interactive_message::Body { text: Some("...".into()) })
    .header(interactive_message::Header { title: Some("...".into()), ..Default::default() })
    .footer(interactive_message::Footer { text: Some("...".into()), ..Default::default() })
    .quoted()
    .await?;
```

This is what every higher-level interactive builder compiles down to
internally.

---

### `ReactionBuilder`

```rust
ctx.send().reaction("👍").await?;
// or use a shortcut:
ctx.send().success().await?; // "✅"
ctx.send().wait().await?;    // "⏳"
ctx.send().failed().await?;  // "❌"
```

Unlike the other builders, `ReactionBuilder` always reacts to **the message
currently being processed** (it reads `chat_jid`, `sender_jid`, and the
message id straight off `ctx`) — there's no way to react to an arbitrary
other message through this builder. It has no `.quoted()` — reactions attach
to a message by definition, so quoting is meaningless here.

> **Heads-up:** `MessageKey.participant` is set to `ctx.info().sender_jid()`
> and `from_me` is hardcoded to `false`. This is correct when reacting to
> someone else's message. If a command ever needs to react to a message the
> bot itself sent (e.g. reacting to its own earlier confirmation), this
> builder would need `from_me: true` and a different participant — that path
> isn't currently supported.

---

## Adding a new message type

Every builder in this module follows an identical shape, which makes adding
a new one mechanical:

1. Define a `struct FooBuilder<'a> { pub ctx: &'a Context, pub quoted: bool, /* type-specific fields */ }`.
2. Add setter methods that take `mut self`, mutate a field, and `return self`.
3. Implement `async fn send(self) -> anyhow::Result<()>` that:
   - builds `context_info` from `self.ctx.info().ctx_info()` if `self.quoted`,
   - uploads any media via `self.ctx.media().upload(...)` if applicable,
   - constructs the `whatsapp::Message` (or delegates to a
     `whatsapp_rust::media::*_message` helper),
   - calls `self.ctx.send().raw(message).await`.
4. Implement `IntoFuture` for the builder (copy-paste from any existing
   builder — the `Box::pin(async move { self.send().await })` body is
   identical everywhere).
5. Wire up a constructor method on `MessageFactory` (or `InteractiveFactory`
   for interactive types) that returns the new builder with sensible
   defaults (`quoted: false`, everything else `None`).

---

## Known gaps / things worth double-checking

- **`extended.rs`** exists as a module (declared in `mod.rs`) but its
  contents weren't available when this document was written — document it
  here once its API is finalized.
- **`AudioBuilder`** setters for duration/mimetype/waveform may not currently
  reach the outgoing `AudioOptions` (see note under [`AudioBuilder`](#audiobuilder)).
- **`ReactionBuilder`** cannot react to a message other than the one
  currently being handled, and cannot react as "the bot's own message" — see
  the note under [`ReactionBuilder`](#reactionbuilder).
