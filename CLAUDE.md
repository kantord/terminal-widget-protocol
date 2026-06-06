# CLAUDE.md — Documentation & RFC style

This repo's headline artifact is **`RFC.md`**, a draft specification for the
Terminal Widget Protocol (TWP), plus `twp-proxy`, a reference **polyfill**. When
writing or editing `RFC.md` (and other prose docs), follow these conventions.
They encode decisions already made — don't relitigate them in passing.

## Framing — the load-bearing decisions

- **Lead with TWP, the concrete protocol.** Do **not** headline the "universal
  envelope" idea. We considered factoring framing/encoding/dispatch into a shared
  envelope and *deliberately decided against building it* — its value depends on
  multi-protocol adoption TWP doesn't need. It survives only as a single
  forward-looking note in Future Work, explicitly "recorded as a possibility, not
  proposed."

- **The bundled `twp-proxy` is a polyfill, not a blueprint — and it is
  NON-NORMATIVE.** It's a deliberately simple, quickly-built renderer that proves
  the protocol from the *application's* side on today's terminals. Never describe
  its architecture (an out-of-band rasterizer, a Kitty-graphics display backend,
  a particular CSS/SVG engine) as how a terminal *should* implement TWP. It
  demonstrates *behavior*, not implementation.

- **Native integration is the intended implementation** — wire TWP into the
  terminal's *existing* rendering pipeline so it reuses the terminal's fonts,
  grid, palette, rasterizer, and screen model. A separate renderer (proxy/library)
  is **also valid**, not merely a stopgap. Be explicit that the ideal tradeoff
  (simplicity / performance / correctness) is **genuinely unknown** until TWP is
  used in practice in specific terminals. Don't fake certainty.

- **KGP (Kitty graphics protocol) is the polyfill's display backend, NOT part of
  TWP.** TWP does not depend on it; a native renderer paints directly and need not
  involve KGP. Never write "TWP uses KGP." (Conceptually KGP is the imperative
  pixel layer *below* TWP's declarative document layer — canvas vs. HTML/CSS.)

## Motivation framing

The motivation is **not** "terminals can't do graphics" or "stuck at the cell
grid." It is that doing graphics *well* is hard and the terminal already solves
the hard parts. Keep these threads:

- Existing graphics (Sixel/KGP/iTerm2) place **opaque pixels bolted on the side** —
  no reflow, theme, text selection, search, or accessibility.
- The terminal owns the things external renderers must re-solve and inevitably
  drift on: **font rendering** (the hardest part), the **layout grid**, the
  **color theme**, and the **screen model** (selection/reflow/a11y).
- **Web tech is a natural fit**: design tokens + color blending (`color-mix()`,
  relative colors) make a coherent UI from a *small* palette — exactly the
  terminal's ~16-color situation.
- TWP is a **low-level substrate for CLI/TUI frameworks**, motivated by the surge
  of rich/interactive terminal apps including **AI agents** — so frameworks don't
  each reinvent renderer + theming + layout.
- Declarativeness is the **mechanism** (it lets the terminal own the rendering),
  not the goal.

## RFC conventions

- Use **BCP 14 / RFC 2119** keywords (MUST / MUST NOT / SHOULD / MAY / …) —
  uppercase only, and only where a real requirement is meant. The boilerplate
  lives in "Conventions and Terminology." Don't sprinkle them casually.
- Preserve the structure: status/version header → Conventions & Terminology →
  numbered sections → Security Considerations → References (normative /
  informative) → worked-example appendix.
- **Section numbers are stable cross-references.** Don't renumber casually; if you
  must, update the `§N` references.
- Label non-binding passages **(informative)**.
- Honesty over salesmanship. Acknowledge limitations (the polyfill's drift, the
  text/widget coexistence limit, unknown tradeoffs). For a spec seeking adoption,
  candor reads as credibility; let the demo screenshots do the selling.

## Accuracy

- **The spec MUST match the implementation.** `twp-proxy/src/protocol.rs` is the
  source of truth for the wire format (node types, style keys, `Dimension` units,
  color functions). Don't document aspirational features as shipped — put them in
  Future Work.
- Established vocabulary (use exactly these spellings):
  - Wire: `ESC _ twp;v=1,c=COLS,r=ROWS ; {compact JSON} ESC \`
  - Payload keys: `S` (scene), `C` (component defs)
  - Nodes: `flex`, `box`, `text`, `mono`, `svg`, `img`, `stack` (+ `$param` /
    `$<name>` components)
  - Cell units: `mcw`, `mch`, `mcmin`, `mcmax` (px and `%` also valid)
  - Colors: `term(fg|bg|0-255)`, `transparent`, `currentColor`, `color-mix(...)`
  - Mono sizing: `scale`, `char-width`, `subscale-n`, `subscale-d`
- The "unknown ⇒ ignore, never fail" rule is *the* forward-compatibility story;
  invoke it consistently (unknown node type, style prop, header key, version).

## Voice

Precise, concise, technical. Concrete examples and tables over abstraction. Short
sentences for normative statements. Em-dashes and parentheticals are fine. Avoid
hype.
