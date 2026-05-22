# twp-proxy — Terminal Widget Protocol hello-world

A Rust binary that wraps your shell in a PTY, intercepts a custom APC
escape sequence (`ESC _ twp ; <payload> ESC \`), renders a shape via the
[takumi](https://crates.io/crates/takumi) layout engine, caches the
rendered PNG by content hash, and emits Kitty Graphics Protocol escape
sequences so the shape appears inline at the cursor position.

Today it understands exactly two payloads:

- `twp;foo` → green box (stands in for a triangle in this hello-world)
- `twp;bar` → red box (stands in for a circle)

The point of this prototype is to prove the end-to-end pipeline

```
shell → PTY → APC interceptor → takumi → content-hash cache → Kitty Graphics → terminal
```

works for one tiny example. Future versions will replace the trivial
payload with a real declarative protocol.

## Requirements

- Rust toolchain (edition 2024, tested with rustc 1.95)
- A terminal that speaks the Kitty Graphics Protocol with Unicode
  placeholders (Kitty, WezTerm with `enable_kitty_graphics`, Ghostty)

## Build

```bash
cd twp-proxy
cargo build --release
```

The binary is `target/release/twp-proxy`.

## Run

```bash
./target/release/twp-proxy zsh    # or bash, or $SHELL
```

You're now in your normal shell, wrapped by the proxy. Try it:

```bash
./demo/hello.sh
```

Or directly:

```bash
printf '\x1b_twp;foo\x1b\\'
printf '\x1b_twp;bar\x1b\\'
```

Each `twp;...` APC is replaced with a rendered widget inline in the
output stream.

## What's inside

- `src/main.rs` — argv/PTY setup, threads for stdin/PTY/SIGWINCH, exit handling
- `src/parser.rs` — byte-level state machine that recognizes APC framing and
  dispatches `twp;` payloads (vte 0.15 doesn't expose APC bytes, so we
  parse them ourselves)
- `src/render.rs` — takumi calls that produce the PNGs
- `src/kitty.rs` — Kitty Graphics transmit + Unicode-placeholder emission
- `src/cache.rs` — content-hash → image-id cache so identical payloads
  skip retransmission

## Anti-goals (intentionally out of scope)

- No real protocol schema. Payloads are literal `foo` / `bar`.
- No real shape rendering — both widgets are colored boxes.
- No SIGWINCH-driven re-render, no tmux passthrough, no theme awareness.
- No CLI flags beyond the shell name.
- No Python kitten wrapper; this is a standalone binary.
