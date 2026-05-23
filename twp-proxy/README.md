# twp-proxy — Terminal Widget Protocol reference implementation

A Rust binary that wraps your shell in a PTY, intercepts the Terminal
Widget Protocol (TWP) APC namespace, renders the requested widget tree
via the [takumi](https://crates.io/crates/takumi) layout engine, and
emits Kitty Graphics Protocol output so the widget appears inline at
the cursor.

```
shell → PTY → TWP parser → JSON → takumi → Kitty Graphics → terminal
```

This crate implements **Phase 1** of TWP. The spec is below.

## Requirements

- Rust toolchain (edition 2024, tested with rustc 1.95)
- A terminal that speaks Kitty Graphics with Unicode placeholders
  (Kitty, WezTerm with `enable_kitty_graphics`, Ghostty)

## Build & run

```bash
cd twp-proxy
cargo build --release
./target/release/twp-proxy zsh    # or bash, or $SHELL
```

Inside the wrapped shell:

```bash
./demo/hello.sh
```

## TWP Phase 1 — protocol spec

TWP is a terminal protocol for sending declarative descriptions of small
structured widgets — boxes, text, flex layouts, reusable components —
that the terminal renders inline in its own font and color scheme,
cell-aligned with the surrounding text.

Phase 1 is the minimum useful subset. It is deliberately small so that
any terminal author can ship support in roughly a week using off-the-
shelf parts (JSON parser + flexbox layout library + their existing
bitmap pipeline).

### 1. Wire framing

Each widget is one APC sequence [5]:

```
ESC _ twp ; <key=val>[,<key=val>]* ; <json-payload> ESC \
```

A single comma-separated header section, then `;`, then the JSON body. The
overall framing — APC envelope, header-then-payload, comma-separated key=val
pairs — follows the same shape established by Kitty's Graphics Protocol [1].
Phase 1 defines exactly three header keys:

| key | meaning                                                     |
|-----|-------------------------------------------------------------|
| `v` | protocol version. Phase 1 = `1`. Required.                  |
| `c` | cell columns — e.g. `c=20`. Required.                       |
| `r` | cell rows — e.g. `r=4`. Required.                           |

Example: `ESC _ twp ; v=1,c=20,r=4 ; {"S": ...} ESC \`

Unknown keys are reserved for future versions and **must be ignored** by
Phase 1 consumers. Senders that need behaviour beyond Phase 1 should
bump `v` and the consumer can decide whether to render or skip.

Both APC terminators are valid: `ESC \` (ST) and `BEL`. Implementations
must accept both, per ECMA-48 [5].

The token `twp` is a reserved vendor identifier within the APC namespace;
other widget protocols sharing the channel should pick different tokens.
Same convention Kitty uses for `G` (graphics) [1] and `_kitty-` (remote
control DCS) [4].

### 2. Payload structure

The payload is a JSON object with up to two protocol-level keys at the
root:

```jsonc
{
  "S": <node>,                // optional: scene to render
  "C": { "<name>": <node> }   // optional: components to register
}
```

- `S` ("scene") is the node tree to draw.
- `C` ("components") is a map of names → component definitions to
  register before the scene renders. The map may be sent alone (to
  pre-register definitions without drawing anything) or combined with
  a scene.
- An empty object `{}` is a valid no-op.

Both keys are case-sensitive and only meaningful at the payload root.
Inside nodes, `n`, `s`, `c` mean node-name, style, children — never
confuse them.

### 3. Node model

Every node is a JSON object:

```jsonc
{
  "n": "<name>",        // required: node type
  "s": { ... },         // optional: style object (Phase 1 vocabulary below)
  "c": [ <node>, ... ], // optional: children
  "t": "...",           // text content (only on "text" nodes)
  "props": { ... }      // params for component invocations
}
```

Phase 1 defines three primitive node names. The node name **is** the
layout algorithm — there is no `display` property. This keeps the
implementer's job legible: "Phase 1 = these three primitives, no more."

- `"flex"` — a container that lays out its children using flexbox.
  Takes the flex-related style properties (`flex-direction`,
  `justify-content`, `align-items`, `gap`).
- `"box"` — a styled rectangle with no layout algorithm. Useful for
  visual leaves (filled shapes, divider lines, dots) and for cases
  where you want only the visual container without flex semantics.
  Children, if any, stack in block flow.
- `"text"` — a text run. The string lives in the `t` field. Text is
  rendered in **the terminal's own font** — this is the headline
  difference between TWP and "just send a PNG."

Any node name beginning with `$` is a **component invocation** (see
§4). Phase 2 will add `"grid"` as a fourth primitive.

### 4. Component model

Components are reusable subtrees with named holes.

A **component definition** is any node tree containing `$param` nodes
as placeholders:

```jsonc
{ "n": "box", "s": { "background": "#0a0", "border-radius": 6 },
  "c": [ { "n": "$param", "name": "label" } ] }
```

A definition is registered by putting it in the payload's `C` map:

```jsonc
{ "C": { "badge": { ...above... } } }
```

A component is **invoked** by using `$<name>` as a node's `n`, with the
hole values supplied in `props`:

```jsonc
{ "n": "$badge", "props": { "label": { "n": "text", "t": "PASS" } } }
```

The invocation node is replaced (at render time) with the def's tree,
substituting each `$param` node with the value from `props` by name.

Rules:

- A `$param` node may appear anywhere a node is allowed. Its value may
  itself be any node tree (or a primitive value, when used at a value
  position inside a style — Phase 2 will define this; Phase 1 only
  requires subtree substitution).
- Params are **lexically scoped** to the enclosing `$call`. Nested
  components do not see their caller's params.
- `$call` recursion must be bounded. Implementations should pick a cap
  (~32 levels is plenty for Phase 1) and reject deeper trees.
- Invoking a component that isn't in `C` (or the implementation's
  pre-registered set) is a **silent no-op** for the affected subtree —
  matching APC's "drop on terminals that don't understand you"
  philosophy.

### 5. Style vocabulary

Phase 1 styles are a small fixed set. Any property outside this list is
ignored.

**Layout** (apply to `flex` containers; ignored elsewhere):

| property            | values                                                          |
|---------------------|-----------------------------------------------------------------|
| `flex-direction`    | `"row"` \| `"column"`                                           |
| `justify-content`   | `"start"` \| `"end"` \| `"center"` \| `"space-between"` \| `"space-around"` \| `"space-evenly"` |
| `align-items`       | `"start"` \| `"end"` \| `"center"` \| `"stretch"`               |
| `gap`               | number (px)                                                     |
| `padding`           | number (px, applies to all sides; valid on any container)       |

**Sizing**:

| property | values                                  |
|----------|-----------------------------------------|
| `width`  | number (px) or string `"N%"`            |
| `height` | number (px) or string `"N%"`            |

**Visual**:

| property        | values                                            |
|-----------------|---------------------------------------------------|
| `background`    | CSS-style color string: `"#RGB"`, `"#RRGGBB"`, `"#RRGGBBAA"` |
| `color`         | same; sets text color (inherited by descendants)  |
| `border-radius` | number (px) or string `"N%"`                      |
| `border`        | `{ "width": <px>, "color": "<color>" }` (solid only) |

**Text** (apply to `text` nodes):

| property      | values                                              |
|---------------|-----------------------------------------------------|
| `font-size`   | number (px)                                         |
| `font-weight` | `"normal"` \| `"bold"` \| number 100–900            |
| `text-align`  | `"left"` \| `"center"` \| `"right"`                 |

That's the entire Phase 1 surface. Five sections, two node primitives,
~12 style properties. Everything else is Phase 2 or later.

### What Phase 1 does not include

Deliberately left for later phases: gradients, shadows, raster/SVG
images, custom fonts, animation, transforms, grid layout, absolute
positioning, query/probe verbs, payload compression. A Phase 1 renderer
may freely cache, memoize, or optimize, but none of that is wire-
visible.

## References

The protocol borrows several established conventions by reference rather
than reinventing them. Citations are non-normative — implementers don't
need to read these to be compliant — but they document where each pattern
comes from and what mental model to bring.

[1] Kitty Graphics Protocol.
    https://sw.kovidgoyal.net/kitty/graphics-protocol/
    — APC envelope and `header;payload` shape (§1); Unicode placeholder
    rendering used internally by this implementation.

[2] Kitty Text Sizing Protocol.
    https://sw.kovidgoyal.net/kitty/text-sizing-protocol/
    — Cell-as-canonical-unit mental model; basis for the planned
    `Ncol`/`Nrow` length grammar.

[3] Kitty Desktop Notifications Protocol.
    https://sw.kovidgoyal.net/kitty/desktop-notifications/
    — Patterns for identifier-keyed updates and reverse-channel callbacks
    being considered for later phases.

[4] Kitty Miscellaneous Escape Codes.
    https://sw.kovidgoyal.net/kitty/misc-protocol/
    — Vendor-token reservation convention; env-var advertisement style
    (`TERM_PROGRAM`, `KITTY_WINDOW_ID`) being considered for capability
    detection.

[5] ECMA-48 / ISO 6429 — Control Functions for Coded Character Sets.
    https://www.ecma-international.org/publications-and-standards/standards/ecma-48/
    — Authoritative definitions for APC, ST, BEL.

[6] takumi — Rust rendering engine for component trees.
    https://crates.io/crates/takumi
    — Layout and rasterization library used by this reference
    implementation. The protocol itself is renderer-agnostic.

## What's inside this crate

- `src/main.rs` — argv/PTY setup, I/O threads, signal handling
- `src/parser.rs` — APC byte-level framing parser
- `src/protocol.rs` — Phase 1 JSON schema (serde types)
- `src/expand.rs` — `$call` + `$param` substitution pass
- `src/render.rs` — protocol-tree → takumi-tree converter + render
- `src/kitty.rs` — Kitty Graphics emission with Unicode placeholders
- `src/cache.rs` — content-hash image-ID cache (implementation detail,
  not part of the protocol)
