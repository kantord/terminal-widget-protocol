# CLAUDE.md — Documentation & RFC style

This repo's headline artifact is **`RFC.md`**, a draft specification for the
Terminal Widget Protocol (TWP), plus `twp-proxy`, a reference **polyfill**. When
writing or editing `RFC.md` (and other prose docs), follow these conventions.
They encode decisions already made — don't relitigate them in passing.

## Respect for prior work (applies everywhere)

This document **builds upon and naturally extends** existing terminal graphics
work — it does not criticize it. Sixel, the Kitty graphics protocol, iTerm2
inline images, OSC 8, the Kitty text-sizing protocol, and TUI frameworks are
**foundations TWP stands on**, and the framing must reflect that:

- Treat existing protocols as **solving their problem well** (pixel transport,
  hyperlinks, text sizing). TWP adds a **complementary layer** (declarative,
  integrated rendering), it does not "fix" or "replace" them.
- When describing the gap TWP fills, frame it as **a different layer / a new use
  case**, never as a deficiency in prior work. They operate at the pixel layer
  _by design_; that is not a flaw.
- Avoid dismissive language: no "opaque blob," "bolted on," "looks foreign,"
  "yet another image escape," etc. Prefer neutral, accurate phrasing ("operates
  at the pixel layer," "renders the final image externally," "a complementary
  declarative layer").
- Credit generously and specifically. `img` keys mirror KGP _for portability_;
  `mono` sizing follows the Kitty text-sizing protocol; OSC 8 is cited as a
  model for community-published adoption. Keep doing this.

The tone is "here is a natural next layer that lets these pieces compose," not
"here is what everyone else got wrong."

## Framing — the load-bearing decisions

- **Lead with TWP, the concrete protocol.** Do **not** headline the "universal
  envelope" idea. We considered factoring framing/encoding/dispatch into a
  shared envelope and _deliberately decided against building it_ — its value
  depends on multi-protocol adoption TWP doesn't need. It survives only as a
  single forward-looking note in Future Work, explicitly "recorded as a
  possibility, not proposed."

- **The bundled `twp-proxy` is a polyfill, not a blueprint — and it is
  NON-NORMATIVE.** It's a deliberately simple, quickly-built renderer that
  proves the protocol from the _application's_ side on today's terminals. Never
  describe its architecture (an out-of-band rasterizer, a Kitty-graphics display
  backend, a particular CSS/SVG engine) as how a terminal _should_ implement
  TWP. It demonstrates _behavior_, not implementation.

- **Native integration is the intended implementation** — wire TWP into the
  terminal's _existing_ rendering pipeline so it reuses the terminal's fonts,
  grid, palette, rasterizer, and screen model. A separate renderer
  (proxy/library) is **also valid**, not merely a stopgap. Be explicit that the
  ideal tradeoff (simplicity / performance / correctness) is **genuinely
  unknown** until TWP is used in practice in specific terminals. Don't fake
  certainty.

- **KGP (Kitty graphics protocol) is the polyfill's display backend, NOT part of
  TWP.** TWP does not depend on it; a native renderer paints directly and need
  not involve KGP. Never write "TWP uses KGP." The useful contrast is
  _imperative pixels vs. a declarative description_ — but keep it at that level
  (see next).

- **Do NOT frame TWP as "HTML/CSS in the terminal," a "document," a "styling
  language," or a "browser."** That oversells the scope (TWP is a small, bounded
  vocabulary — a handful of node types and style properties, no scripting/DOM/
  network) and it actively triggers the terminal-minimalism objection ("they
  want to put a browser in my terminal"). Frame TWP as a **small, optional,
  ignorable widget primitive** the terminal can implement a subset of. Address
  the "don't bloat the terminal" concern directly: it's a standard APC sequence
  unaware terminals swallow (§4), partial support is fine via "unknown ⇒ ignore"
  (§10), and it reuses the terminal's existing rendering rather than adding a
  new subsystem. (Borrowing _concepts_ from CSS — flexbox, `color-mix()` — for
  familiarity is fine; claiming TWP _is_ CSS/HTML is not.)

## Motivation framing

The motivation is **not** "terminals can't do graphics" or "stuck at the cell
grid." It is that doing graphics _well_ is hard and the terminal already solves
the hard parts. Keep these threads:

- Existing graphics (Sixel/KGP/iTerm2) place **opaque pixels bolted on the
  side** — no reflow, theme, text selection, search, or accessibility.
- The terminal owns the things external renderers must re-solve and inevitably
  drift on: **font rendering** (the hardest part), the **layout grid**, the
  **color theme**, and the **screen model** (selection/reflow/a11y).
- **Web tech is a natural fit**: design tokens + color blending (`color-mix()`,
  relative colors) make a coherent UI from a _small_ palette — exactly the
  terminal's ~16-color situation.
- TWP is a **low-level substrate for CLI/TUI frameworks**, motivated by the
  surge of rich/interactive terminal apps including **AI agents** — so
  frameworks don't each reinvent renderer + theming + layout.
- Declarativeness is the **mechanism** (it lets the terminal own the rendering),
  not the goal.

## RFC conventions

- Use **BCP 14 / RFC 2119** keywords (MUST / MUST NOT / SHOULD / MAY / …) —
  uppercase only, and only where a real requirement is meant. The boilerplate
  lives in "Conventions and Terminology." Don't sprinkle them casually.
- Preserve the structure: status/version header → Conventions & Terminology →
  numbered sections → Security Considerations → References (normative /
  informative) → worked-example appendix.
- **Section numbers are stable cross-references.** Don't renumber casually; if
  you must, update the `§N` references.
- Label non-binding passages **(informative)**.
- Honesty over salesmanship. Acknowledge limitations (the polyfill's drift, the
  text/widget coexistence limit, unknown tradeoffs). For a spec seeking
  adoption, candor reads as credibility; let the demo screenshots do the
  selling.

## Accuracy

- **The spec MUST match the implementation.** `twp-proxy/src/protocol.rs` is the
  source of truth for the wire format (node types, style keys, `Dimension`
  units, color functions). Don't document aspirational features as shipped — put
  them in Future Work.
- Established vocabulary (use exactly these spellings):
  - Wire: `ESC _ twp;v=1,c=COLS,r=ROWS ; {compact JSON} ESC \`
  - Payload keys: `S` (scene), `C` (component defs)
  - Nodes: `flex`, `box`, `text`, `mono`, `svg`, `img`, `stack` (+ `$param` /
    `$<name>` components)
  - Cell units: `mcw`, `mch` (px and `%` also valid). There is **no** `mcmin`/
    `mcmax` — they were redundant (on a taller-than-wide cell `mcmin` ≡ `mcw`);
    a pixel-square element is just the same unit on both axes.
  - Colors: `term(fg|bg|0-255)`, `transparent`, `currentColor`, `color-mix(...)`
  - Mono sizing: `scale`, `char-width`, `subscale-n`, `subscale-d`
- **Grid stability (load-bearing).** Layout (`width`/`height`/`padding`/`gap`/
  `flex-*`, in cell units) positions content on the grid; paint (`background`,
  `border`, `border-radius`, effects) colours pixels and MUST NOT move a node,
  its content, or its siblings — it MAY bleed outside the box. So a `border` is
  non-displacing; content leaves the grid only via an explicit sub-cell `px`
  value. The polyfill realises the typed `border` as a CSS `outline` (paint, no
  layout, distinct from `box-shadow`) — a polyfill detail, never spec'd as how a
  terminal "should" implement it.
- The "unknown ⇒ ignore, never fail" rule is _the_ forward-compatibility story;
  invoke it consistently (unknown node type, style prop, header key, version).

## Docs figures & examples pipeline

Figures and worked examples in `RFC.md` are **generated, not hand-pasted**, so
they can't drift from the implementation:

- Each example is one JSON source in `examples/*.json`. `mdsh` runs `just`
  recipes from HTML-comment directives
  (``<!-- `> just example NAME COLS ROWS "THEME"` -->``) and inserts, in place,
  **the JSON code block + the rendered image** between
  `<!-- BEGIN mdsh -->`/`<!-- END mdsh -->` sentinels.
- `twp-render` (in `src/bin/render.rs`) renders a scene to PNG **in-process**
  (no kitty/Xvfb) — `--in file --cols N --rows N` or `--demo NAME`, plus
  `--theme`. Pixels equal what the proxy transmits.
- Recipes: `just example` (JSON + image), `just figure DEMO OUT` (image only,
  for big scenes), `just docker-hero` (the §1 two-theme table). `just docs` runs
  mdsh over the whole file.
- **Regenerate with `just docs`** after editing an example or a demo. Keep
  `examples/*.json` in dprint's json style (prek does this) so the embedded copy
  matches and dprint/mdsh don't ping-pong.
- The single exception is the native-ANSI-vs-`term()` figure (§8.2): its left
  half is the terminal's own rendering, so it needs the kitty harness
  (`scripts/gen-figures.sh`), not `twp-render`.
- Image _pixels_ are font/rasterizer dependent, so they are regenerated-and-
  committed, not verified in CI. The JSON blocks are deterministic and could be
  `mdsh --frozen`-checked.

## Voice

Precise, concise, technical. Concrete examples and tables over abstraction.
Short sentences for normative statements. Em-dashes and parentheticals are fine.
Avoid hype.
