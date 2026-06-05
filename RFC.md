# Terminal Widget Protocol (TWP) — Draft RFC

**Status:** Draft / Request for Comments
**Version:** Protocol `v=1` (Phase 1)
**Reference implementation:** `twp-proxy` (this repository)

---

## Abstract

The Terminal Widget Protocol (TWP) is an escape-sequence protocol that lets a
program render **declarative, themeable widgets** inline in a terminal — flexbox
layout, real fonts at real sizes, vector graphics, images, gradients, shadows,
and rounded corners — by sending a single JSON scene description wrapped in an
APC control sequence.

Where the Kitty graphics protocol gives the terminal a *canvas* (here are the
pixels, place them at this cell), TWP gives it a *document* (here is a widget
tree; you lay it out and rasterize it). The relationship is intentionally that
of **HTML/CSS to `<canvas>`**: a declarative layer on top of the imperative
pixel layer.

Two properties make TWP more than "yet another image escape":

1. **Cell-native layout.** Sizes are expressed in *monospace cell units*, so a
   widget aligns to the character grid on any terminal regardless of font,
   size, or DPI — not just the one it was authored on.
2. **Theme-reactive color.** Colors can be drawn from the terminal's own
   palette (`term(fg)`, `term(2)`, …) and derived from it (`color-mix(...)`),
   so the same widget re-tones itself to the user's color scheme — light or
   dark — with no per-theme code.

TWP is transport-framed with the standard ECMA-48 APC mechanism, so terminals
that do not implement it silently ignore TWP messages.

---

## 1. Motivation

### 1.1 The terminal is stuck at the cell grid

A terminal emulator presents a grid of fixed-size character cells. Decades of
extensions have chipped at this — 256/truecolor SGR, Sixel, the Kitty graphics
protocol, the Kitty text-sizing protocol (OSC 66), OSC 8 hyperlinks — but the
*layout and styling* model has never moved beyond "characters in cells."
Anything richer (a chart, a status card, a progress bar that isn't made of
block glyphs) requires either leaving the terminal for a browser/GUI, or
hand-approximating the visual with Unicode block/braille characters.

### 1.2 The gap is a *declarative* layer, not more pixels

The pixel problem is largely solved: the Kitty graphics protocol can put
arbitrary bitmaps on the screen, anchored to the cell grid via Unicode
placeholders. But it is *imperative and low-level* — the sender must produce the
final pixels and manage placement. There is no shared, declarative way to say
"a row containing a label, a flexible spacer, and a right-aligned badge, themed
to my terminal colors, that lays out correctly at any font size." Every
application that wants rich output must build its own renderer and ship pixels.

TWP fills that gap: a declarative widget description the terminal (or a proxy)
lays out and rasterizes, reusing the existing graphics protocol for display.

### 1.3 Why now

Smooth, anti-aliased rendering with real fonts is cheap (pure software
rasterization works over SSH with no GPU). The Kitty graphics protocol gives a
portable display primitive. The Kitty text-sizing protocol established that
terminals will accept richer text geometry. The missing piece is a declarative
vocabulary, which is what this document specifies.

---

## 2. Design Goals & Non-Goals

**Goals**

- **Declarative.** Senders describe *what* to show, not *how* to draw it.
- **Cell-native.** Layout aligns to the character grid on any terminal.
- **Theme-reactive.** Color can derive from the terminal palette.
- **Degrade safely.** Unaware terminals ignore TWP; unknown features within a
  TWP message are dropped, never fatal.
- **Reuse what exists.** Framing is plain APC; display reuses the Kitty graphics
  protocol. TWP invents only the declarative layer.
- **Implementable as a proxy *or* natively.** A reference proxy renders TWP for
  any Kitty-graphics-capable terminal today; terminals may implement it
  directly later.

**Non-Goals (Phase 1)**

- Not a general GUI toolkit (no event model, no focus, no animation timeline in
  the wire format — though CSS effects and SVG animation are not precluded).
- Not a replacement for the Kitty graphics protocol; TWP *uses* it.
- No interactivity/hit-testing in Phase 1 (see §11, Future Work).

---

## 3. Architecture

```
   application                terminal-side renderer            display
  ┌───────────┐   APC + JSON  ┌──────────────────────┐  KGP   ┌─────────┐
  │  sender   │ ────────────► │  TWP renderer         │ ─────► │ terminal│
  │ (TUI/CLI) │   twp;…;{…}   │  (proxy or native)    │ images │  screen │
  └───────────┘               └──────────────────────┘        └─────────┘
```

A **TWP renderer** intercepts TWP escape sequences in the byte stream, lays out
and rasterizes the scene to a bitmap, and displays it. The reference
implementation, `twp-proxy`, is a PTY proxy: it sits between an application and a
Kitty-graphics-capable terminal, renders each scene to a PNG, transmits it with
the Kitty graphics protocol using Unicode placeholders, and emits a `c×r`
placeholder-cell grid where the widget appears (§9). A terminal could perform
the same steps internally.

The renderer also queries the terminal for context it needs (§7, §8): the cell
pixel size (to resolve cell units) and the active palette (to resolve
`term(...)` colors).

---

## 4. Wire Format

### 4.1 Framing

A TWP message is a single ECMA-48 **Application Program Command (APC)**:

```
ESC _  twp;<header>;<payload>  ESC \
```

- `ESC _` (`0x1B 0x5F`) — APC introducer. The 8-bit form (`0x9F`) MAY be
  accepted; senders SHOULD emit the 2-byte form for 7-bit/UTF-8 safety.
- `twp;` — the TWP namespace prefix. A renderer dispatches only APC sequences
  beginning with this prefix; all other APCs pass through untouched.
- `<header>` — comma-separated `key=value` control fields (§4.2).
- `;` — separates header from payload.
- `<payload>` — a single compact (single-line) JSON document (§4.3).
- `ESC \` (`0x1B 0x5C`, ST) — string terminator. The 8-bit form (`0x9C`) MAY be
  accepted.

Because APC content is opaque to the terminal, a terminal that does not
implement TWP swallows the entire sequence and displays nothing — TWP's
baseline graceful-degradation property.

### 4.2 Header fields

| Key | Meaning | Required |
|-----|---------|----------|
| `v` | Protocol version. This document specifies `v=1`. | yes |
| `c` | Cell **columns** the widget occupies. | yes |
| `r` | Cell **rows** the widget occupies. | yes |

Unknown header keys MUST be ignored (forward-compatibility). A renderer that
does not support the declared `v` MUST ignore the message.

`c` and `r` declare the widget's **cell footprint** — the rectangle of character
cells the rendered image will occupy. They let the renderer reserve grid space
and size the output without parsing the payload.

Example header: `twp;v=1,c=40,r=6;`

### 4.3 Payload (JSON)

The payload is a compact JSON object. Compact (no raw newlines; control
characters `\u`-escaped) JSON is APC-safe by construction — it can contain no
raw `ESC` — so it rides the APC channel verbatim with no further encoding.

Top-level keys:

| Key | Meaning |
|-----|---------|
| `S` | **Scene** — the root node of the widget tree (§5). |
| `C` | **Components** — a map of `name → node` definitions for reuse (§6). |

Both are optional; a payload with neither is a no-op. A payload with only `C`
registers definitions (renderers MAY treat definitions as per-message).

Minimal example (full message):

```
ESC _ twp;v=1,c=8,r=1;{"S":{"n":"mono","t":"hello"}} ESC \
```

---

## 5. The Scene Graph

A **node** is a JSON object:

```json
{ "n": "<type>", "s": { …style… }, "c": [ …children… ], "t": "<text>" }
```

| Field | Meaning |
|-------|---------|
| `n` | Node type (below). Required. |
| `s` | Style object (§7, §8). Optional. |
| `c` | Array of child nodes. Optional. |
| `t` | Text/source content; meaning depends on type. Optional. |
| `img` | Image source, for `img` nodes (§5.6). Optional. |
| `name`, `props` | Component machinery (§6). Optional. |

### 5.1 `flex` — flex container

A CSS-flexbox container. Honors `flex-direction`, `justify-content`,
`align-items`, `gap`, plus sizing/visual style. Children flow per the flex
algorithm. This is the primary layout primitive.

### 5.2 `box` — block container

A styled block container (no flex layout). Used for solid fills, spacers, bars,
dots — anything that is "a rectangle with style."

### 5.3 `text` — proportional text

A run of text (`t`) rendered in a **proportional** font at `font-size`. For
prose, headings, captions — content that is *not* meant to align to the cell
grid.

### 5.4 `mono` — monospace-grid text

A run of text (`t`) where **each character occupies exactly one (or `scale`)
character cell**, laid out on the grid rather than by the font's advance widths.
This is the primitive that keeps text aligned with surrounding cells.

`mono` honors the text-sizing parameters (§7.4), mirroring the Kitty text-sizing
protocol: `scale` (an `s×s`-cell block per glyph), `char-width` (cells per
glyph), and `subscale-n`/`subscale-d` (a fractional glyph size within the cell
block).

### 5.5 `svg` — inline vector graphics

`t` carries inline SVG markup. The renderer rasterizes it into the node's box.
Enables smooth curves, arcs, gauges, sparklines, and gradient fills that the box
model cannot express. SVG `fill`/`stroke` accept `term(...)` colors and
`currentColor` (§8).

### 5.6 `img` — bitmap image

An `img` node carries an `img` object describing a bitmap, with keys
intentionally mirroring the Kitty graphics protocol so the same source
description is portable:

| Key | Meaning |
|-----|---------|
| `f` | Format: `100` = encoded (PNG, default), `32` = RGBA, `24` = RGB. |
| `t` | Transmission: `"d"` = direct base64 in `d`; `"f"` = file at `path`. |
| `s`, `v` | Pixel width / height (required for raw `f=32`/`f=24`). |
| `d` | Base64 payload. |
| `path` | Filesystem path (for `t=f`). |

The node's `border-radius` clips the image (e.g. circular avatars).

### 5.7 `stack` — z-layered overlay

Children are painted as full-bleed layers, later children on top — a z-order
overlay. Used for scrims over images, badges on corners, and floating popovers.
Each layer occupies the stack's full box; position within a layer is achieved
with a nested `flex`.

### 5.8 Unknown node types

A renderer encountering an unknown `n` MUST NOT fail; it SHOULD render nothing
(or an empty box) for that node and continue. This makes new node types
forward-compatible.

---

## 6. Components (`C` / `$`)

To avoid repeating subtrees, a payload may define reusable components in `C` and
invoke them in the scene:

- A definition is a node tree containing **holes**: nodes with `n: "$param"` and
  a `name`, marking where invocation-supplied content goes.
- An **invocation** is a node whose type is `$<name>` (referring to a key in
  `C`), carrying a `props` map. Each prop fills the matching `$param` hole.
- For ergonomics, a prop given as a bare string is wrapped in a `text` node; a
  prop given as an object is used as a full node tree.

Components are expanded before layout. They are a convenience for senders (e.g.
a table row template); the expanded tree is what §5 describes.

---

## 7. Style — Sizing & Layout

Style is a JSON object under `s`. Phase 1 defines a typed vocabulary; any
property not listed is treated as a raw CSS declaration passed to the rasterizer
(§7.5). Unknown/unparseable declarations are dropped.

### 7.1 Layout (on `flex`)

`flex-direction` (`row`|`column`|…), `justify-content`, `align-items`, `gap`,
`padding`. Values follow CSS semantics.

### 7.2 Sizing

`width`, `height`, `border-radius` take a **length** (§7.3). `gap` and `padding`
likewise.

### 7.3 Lengths and cell units

A length is either a bare number (**pixels**) or a string with a unit:

| Form | Unit | Resolves to |
|------|------|-------------|
| `42` | pixels | `42px` (escape hatch for sub-cell cosmetics) |
| `"50%"` | percent | 50% of the parent's corresponding axis |
| `"3mcw"` | monospace **cell width** (x) | `3 · px_per_col` |
| `"2mch"` | monospace **cell height** (y) | `2 · px_per_row` |
| `"1mcmin"` | cell **min** | `1 · min(px_per_col, px_per_row)` |
| `"1mcmax"` | cell **max** | `1 · max(px_per_col, px_per_row)` |

**Cell units are TWP's native length unit and the key to portability.** The
character cell is *anisotropic* (typically ~1:2, taller than wide) and its pixel
size varies per terminal (font, size, DPI). A widget sized in pixels aligns to
the grid only on the terminal it was authored on; a widget sized in cell units
aligns *everywhere*, because the renderer resolves `mcw`/`mch` against the live,
per-terminal cell size.

Because the two axes are independent, there are two base units (`mcw`, `mch`).
For elements that must be *square in pixels* despite the anisotropic cell —
icons, status dots, circular avatars — `mcmin` gives the largest square that
*fits* within a cell (like `object-fit: contain`) and `mcmax` the smallest that
*covers* it (like `cover`). `N·mcmin` scales those square graphics with the grid.

Guidance: use cell units for layout (columns, gaps, padding, bars) and for SVG
node boxes (the SVG `viewBox` preserves internal proportion); use `mcmin` for
square/round graphics; reserve pixels for genuinely sub-cell cosmetics (a 1px
border).

### 7.4 Mono text sizing

On `mono` nodes (mirroring the Kitty text-sizing protocol):

| Key | Meaning |
|-----|---------|
| `scale` | Each glyph occupies a `scale × scale` block of cells. |
| `char-width` | Cells per glyph horizontally. |
| `subscale-n` / `subscale-d` | Glyph drawn at `n/d` of the cell block (a finer sub-grid). |

### 7.5 The passthrough rule

Any style key not in the typed vocabulary is forwarded to the rasterizer as a
CSS `property: value` declaration. This is how effects beyond Phase 1's named
set — `text-shadow`, `opacity`, `box-shadow`, `-webkit-text-stroke`,
`text-decoration`, `background-image: linear-gradient(...)`, `flex-grow`,
`aspect-ratio`, etc. — work with no per-property protocol changes. A declaration
the renderer cannot parse is silently dropped, preserving forward-compatibility.
`term(...)` colors and cell units are resolved inside passthrough values too.

---

## 8. Style — Color & Theming

Color-valued properties (`color`, `background`, SVG `fill`/`stroke`, borders,
gradients, …) accept:

### 8.1 Literal colors

CSS hex (`#rrggbb`, `#rgb`, `#rrggbbaa`) and the keyword `transparent`. A
transparent background composites the widget over the terminal cells beneath it.

### 8.2 Terminal palette — `term(...)`

`term(...)` draws from the terminal's *own* color scheme:

| Form | Meaning |
|------|---------|
| `term(fg)` | Default foreground. |
| `term(bg)` | Default background. |
| `term(0)` … `term(15)` | The 16 ANSI palette colors (theme-dependent). |
| `term(16)` … `term(255)` | The 256-color cube / grayscale ramp (fixed formula). |

The renderer learns the palette by querying the terminal (the reference proxy
uses OSC 4 / OSC 10 / OSC 11). Because indices 0–15 are theme-defined,
`term(2)` is "this user's green," not a fixed RGB — so a widget built on
`term(...)` adopts the user's color scheme automatically.

### 8.3 Derived colors

Standard CSS color functions are accepted, most usefully `color-mix()`:

```
color-mix(in srgb, term(2) 15%, term(bg))
```

This expresses *derived* theme tones — an "added line" tint as the green accent
mixed lightly into the background, "muted" text as the foreground pulled toward
the background, elevated surfaces as subtle lifts off `term(bg)`. A complete UI
can thus derive **every** color from the palette and re-tone itself to any
theme, light or dark, with no per-theme code. Relative color syntax
(`rgb(from term(1) r g b / .3)`) is likewise accepted.

### 8.4 `currentColor` (SVG)

Within an `svg` node, `currentColor` resolves to the node's `color`, letting
vector art inherit a (possibly `term()`-derived) theme color.

---

## 9. Rendering & Display Model

A renderer processes a TWP message as:

1. **Parse** the APC header and JSON payload; expand components (§6).
2. **Resolve** cell units against the queried cell pixel size and `term(...)`
   colors against the queried palette.
3. **Lay out and rasterize** the scene into a bitmap sized to `c × r` cells.
4. **Display** the bitmap anchored to a `c × r` block of character cells.

The reference proxy implements step 4 with the **Kitty graphics protocol using
Unicode placeholders**: it transmits the image (`a=T,f=100,U=1,c=…,r=…`) and
emits a `c × r` grid of `U+10EEEE` placeholder cells (image id encoded in the
cell foreground color, row/column via combining diacritics). The terminal paints
the transmitted image into exactly those cells. A native implementation may use
any equivalent mechanism.

**Cell footprint.** The widget occupies exactly the `c × r` cells declared in
the header. Sizing the scene root to `width:100%; height:100%` fills that box;
the cell-unit system (§7.3) ensures internal elements align to the same grid.

**Coexistence (informative).** In the reference proxy, a placeholder-image
widget and ordinary printed text do not currently share the same screen region
cleanly; widgets are best placed in regions the application manages as widget
real estate. A tighter cell-by-cell coexistence model (analogous to
`ratatui-image`) is future work (§11).

---

## 10. Compatibility, Degradation & Versioning

TWP degrades at three levels:

1. **Wrapper-unaware terminal.** Any terminal that does not implement TWP
   swallows the APC (standard ECMA-48 behavior) and shows nothing. (An optional
   sender-supplied plain-text rendering printed *outside* the APC could let such
   terminals show a fallback; see §11.)
2. **Unknown node type.** Rendered as nothing; the rest of the scene renders
   (§5.8).
3. **Unknown style property / value.** Dropped; the rest of the style applies
   (§7.5).

**Versioning.** `v=1` identifies this protocol revision. Renderers MUST ignore
messages whose `v` they do not support. New *additive* features (node types,
style properties, header keys) do not require a version bump because each
degrades individually; `v` increments only on incompatible changes.

This "unknown ⇒ ignore, never fail" rule is the protocol's entire
forward-compatibility story and applies uniformly at every layer.

---

## 11. Security Considerations

- **Resource use.** A scene can request large images, deep trees, or large
  rasterization targets. Renderers SHOULD bound payload size, tree depth, image
  dimensions, and total rasterized area, and SHOULD rate-limit messages.
- **File access (`img` `t=f`/`path`).** Reading arbitrary filesystem paths named
  by an untrusted sender is dangerous. Renderers SHOULD restrict or disable
  path-based image sources, or sandbox them, especially over SSH or when the
  sender is not the local user.
- **SVG.** SVG can reference external resources and is a historically rich
  attack surface. Renderers MUST disable external entity/resource loading and
  scripting, and SHOULD use a hardened, static SVG rasterizer.
- **Palette/cell queries.** The renderer emits terminal queries (OSC 4/10/11,
  cell-size) and reads responses; it MUST parse these defensively and time out
  rather than block.
- **Untrusted output.** As with any escape-sequence feature, programs that emit
  attacker-controlled bytes can emit TWP. The "unknown ⇒ ignore" rule limits
  blast radius, but the resource and file-access bounds above are essential.

---

## 12. Relationship to Prior Art

- **Kitty graphics protocol.** TWP's display layer; TWP is the declarative layer
  above it (document vs. canvas). `img` source keys deliberately mirror it.
- **Kitty text-sizing protocol (OSC 66).** TWP's `mono` sizing (`scale`,
  `char-width`, `subscale`) mirrors it, generalized into a layout context.
- **Sixel / iTerm2 inline images.** Alternative pixel-display primitives; TWP
  could target them as display backends.
- **CSS / flexbox / SVG.** TWP's style and layout semantics are intentionally a
  subset of CSS, so the model is familiar and the passthrough rule (§7.5) can
  borrow the rasterizer's full CSS engine.
- **TUI frameworks (ratatui, etc.).** TWP is complementary: a TUI could emit a
  TWP scene for a region instead of (or alongside) cell characters, gaining
  sub-cell rendering while keeping the cell grid as the coordinate system.

---

## 13. Open Questions & Future Work

- **Chunking.** Large payloads (image-bearing scenes) may exceed terminal APC
  length limits. A continuation mechanism (à la Kitty's `m=1`) is the main
  realistic addition; deferred until needed.
- **Optional transfer encoding.** A compact-JSON payload is already APC-safe and
  needs no encoding. A future optional `enc=` field could allow compressed
  (`zlib`+base64) payloads for bandwidth, with "unknown enc ⇒ ignore." Not part
  of Phase 1.
- **Plain-text fallback.** A standard way to attach an out-of-band plain-text
  rendering for wrapper-unaware terminals (§10).
- **Interactivity.** A cell→node hit map plus an input convention would enable
  buttons, hover, and focus. Out of scope for Phase 1, but the declarative tree
  is a natural substrate for it.
- **Capability negotiation.** A query/response for "does this terminal support
  TWP `v`, and which features?" so senders can choose between a TWP rendering
  and a degraded one without guessing.
- **Generalized envelope (note).** TWP frames on plain APC like its peers. The
  framing/encoding/dispatch concerns *could* be factored into a shared
  envelope reused by multiple terminal-extension sub-protocols; this is recorded
  as a possibility, not proposed here — its value depends on multi-protocol
  adoption that TWP alone does not require.

---

## Appendix A — Worked Examples

**A.1 A themed status pill (cell-native, theme-derived):**

```json
{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center",
  "width":"100%","height":"100%","background":"term(bg)"},
 "c":[{"n":"flex","s":{"background":"color-mix(in srgb, term(2) 18%, term(bg))",
   "border-radius":"0.45mcmin","padding-left":"0.6mcw","padding-right":"0.6mcw"},
   "c":[{"n":"mono","t":"running","s":{"color":"term(2)"}}]}]}}
```

**A.2 A flex-grow media bar** (fixed glyph + growing track + fixed time):

```json
{"S":{"n":"flex","s":{"flex-direction":"row","align-items":"center",
  "gap":"0.9mcw","width":"100%","height":"100%"},
 "c":[
  {"n":"mono","t":"▶","s":{"color":"term(4)"}},
  {"n":"flex","s":{"flex-grow":1,"height":"0.3mch","background":"term(8)",
    "border-radius":"0.2mcmin"},
   "c":[{"n":"box","s":{"width":"40%","height":"0.3mch","background":"term(4)",
     "border-radius":"0.2mcmin"}}]},
  {"n":"mono","t":"1:23","s":{"color":"term(fg)"}}
 ]}}
```

Full message form: `ESC _ twp;v=1,c=34,r=3;<json> ESC \`.

---

*This is a first draft for discussion. Section numbering, key names, and the
exact unit/keyword spellings are subject to change based on review.*
