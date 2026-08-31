# Deep Review — Terminal Widget Protocol (TWP) Draft RFC

_Reviewed artifact: `RFC.md`. Synthesized from eight independent, fact-checked
review passes (accuracy, consistency, soundness, completeness, prior-art,
framing, editorial, adoption). Every issue below was verified against the
source; refuted claims were dropped. Where a finding was only partly confirmed,
that is stated._

---

## 1. Executive summary

This is a strong, unusually honest draft that is **not yet publishable as-is**,
but is close: the fixes are concentrated and mostly mechanical. The core wire
vocabulary in §4–§9 matches the reference implementation
(`twp-proxy/src/protocol.rs`) faithfully — node types, payload keys, header
keys, cell units, color functions, and mono sizing all line up, and both
appendix examples render cleanly — so the spec's technical spine is sound. The
framing discipline the project set for itself is genuinely well executed: KGP is
credited as the polyfill's backend and never as part of TWP, prior art is
treated as a foundation rather than a deficiency, native integration is
presented as the intended implementation with the tradeoff candidly marked
"genuinely unknown," and the "universal envelope" idea is correctly confined to
a single non-headlined Future-Work note.

The three things that most need fixing, in order: (1) **the Abstract** — it
contains four outright typos (`proposol`, `propgrams`, two wrong `form`/`from`),
is roughly 3× too long, and promises semantic "button/clickable" intent the wire
format does not yet carry, which is a credibility leak on the first page a
reviewer reads; (2) **two undocumented but actively-used typed style keys**
(`border` object, and the typed text fields
`font-size`/`font-weight`/`text-align`) — the source of truth defines them and
the demos emit them, but an implementer reading the spec cannot reproduce them;
(3) **the adoption surface is too large** — the §7.5 CSS-passthrough rule
effectively conscripts a full CSS engine into conformant behavior, undercutting
the "small, bounded vocabulary" claim, and **terminal multiplexers (tmux/screen)
are never mentioned** despite the polyfill being a KGP-emitting PTY proxy — the
single objection a terminal maintainer raises first.

None of the confirmed issues are wire-format-breaking. There are no critical
bugs. The most consequential _technical_ finding (the "APC-safe by construction"
claim vs. accepting 8-bit C1 ST/APC forms) is real as a spec-clarity
inconsistency but does **not** break the reference parser, which only triggers
on `0x1B`. Fix the Abstract, document the typed keys, repair the two broken
`§11`→`§13` cross-references and the `ratatouille` misspelling, and the draft is
publishable. The deeper strategic work — narrowing the normative Phase-1 ask —
is what determines whether it gets _adopted_.

---

## 2. What's strong

- **Spec ↔ implementation fidelity on the core.** The wire vocabulary is
  accurate: `flex`/`box`/`text`/`mono`/`svg`/`img`/`stack` (+
  `$param`/`$<name>`), payload keys `S`/`C`, header keys `v`/`c`/`r`, cell units
  `mcw`/`mch`/`mcmin`/`mcmax`, color functions
  `term()`/`color-mix()`/`currentColor`/`transparent`, and mono sizing
  `scale`/`char-width`/`subscale-n`/`subscale-d` all match `protocol.rs` and
  `render.rs` and are spelled identically throughout. Both appendix examples
  render. This is the hard part and it is done well.
- **The "unknown ⇒ ignore, never fail" rule is uniform and well-implemented.**
  Unknown node types, style props, components, and protocol versions all degrade
  rather than fail — in both the prose and the parser. This is the document's
  forward-compatibility story and it is invoked consistently.
- **Genuine candor about gaps.** Interactivity, accessibility vocabulary,
  chunking, capability negotiation, and the polyfill's text/widget coexistence
  limit are all explicitly deferred to §13 with honest framing, and §3.4 openly
  states the native-vs-proxy tradeoff is "genuinely unknown." For a spec seeking
  adoption, this candor reads as credibility.
- **Framing discipline.** KGP is attributed to the polyfill everywhere
  (Conventions glossary, §3.2/§3.3, §12, References) and the document never
  asserts "TWP uses KGP." Native integration is the intended implementation; the
  proxy is a non-normative polyfill. The "universal envelope" survives only as a
  single forward-looking Future-Work note. Dismissive vocabulary ("opaque blob,"
  "bolted on," "yet another image escape") is avoided in the prose — §1.1
  actively disclaims that the pixel layer is "not a limitation of those
  protocols."
- **BCP-14 keyword discipline.** Every MUST/SHOULD/MAY sits at a real
  requirement point; the boilerplate is correctly placed in Conventions; the
  keywords are not sprinkled casually.
- **Generous, mostly accurate prior-art crediting.** KGP, the Kitty text-sizing
  protocol, OSC 8, Sixel, iTerm2 inline images, and CSS/flexbox/SVG are credited
  specifically and positioned as complementary foundations.
- **RFC structure.** Status header → Conventions → numbered sections → Security
  → References (normative/informative) → worked-example appendix. The skeleton
  follows convention well, and section numbers read as stable cross-references.

---

## 3. Critical & major issues

There are **no critical (wire-breaking) issues**. The items below are the
majors, grouped by theme and ordered by severity then impact. Where multiple
reviewers raised the same point, findings are merged.

### 3.1 (Major) Undocumented typed style keys the source-of-truth defines and the demos use

**Location:** §7/§8 (no definition); `protocol.rs:123,240–244`;
`render.rs:972–974,1025–1054`; `demos.rs:830,864`. **Raised by:** Accuracy
(border object, confirmed major) + Accuracy dimension summary
(`font-size`/`font-weight`/`text-align`).

**Problem.** `protocol.rs` defines a typed `border` style field —
`pub border: Option<Border>` where
`struct Border { pub width: f32, pub color: String }` — rendered as a solid
border on all four sides, with the source comment "Phase 1 supports no other
border styles." It is a real wire key (not part of the `extra` passthrough map)
and the demos emit it directly (`"border":{"width":1,"color":"#30363d"}`, plus
the avatar ring). The spec mentions `border` only once in passing, as a
color-valued category (§8 line 647), and never specifies the object shape, its
required `width`/`color` sub-keys, or the solid-only Phase-1 constraint. Per
CLAUDE.md, `protocol.rs` is the source of truth and the spec MUST match it. The
same gap applies to the other typed text fields the dimension summary flagged
(`font-size`, `font-weight`, `text-align`): heavily used, typed in the source,
never documented as style keys in §7–§8. A worse-than-silent failure mode:
§7.5's passthrough rule would mislead a reader into thinking `border` is plain
CSS, when it is in fact a typed object.

**Fix.** Document the typed `border` key in §7/§8: shape
`{ "width": <px number>, "color": <color> }`, solid-only in Phase 1, applied to
all four edges. Document `font-size`, `font-weight`, and `text-align` as typed
style keys with their value sets. Optionally note the CSS longhands
(`border-width`/`border-style`/`border-color`) also work via passthrough, but
mark the typed object as the documented Phase-1 form.

### 3.2 (Major) The Abstract: typos, length, and an over-promise the wire format can't keep

**Location:** Abstract, lines 8–71 (typos at 3, 15, 37, 60; semantic claims at
21–27; closing caveat 67–71). **Raised by:** Editorial (typos, length), Framing
(credibility leaks), Completeness (semantic over-promise), Adoption
(selectability claim unobservable).

**Problem.** This is the first page a reviewer reads and it is the weakest.
Three outright typos sit in the first two paragraphs — `proposol` (line 3, in
the Status line), `propgrams` (line 15), and `form` for `from` (line 60) — plus
an earlier inverted `form`/`from` in the body. The Abstract is roughly 3× the
length an abstract should be (it runs through line 71, including a "distinguish
it from the following things" list that belongs in §1). It promises semantic
intent — "marking a node as a button so assistive software can announce it as a
clickable region" — that the **Phase-1 wire format does not yet carry**; that is
aspirational and per CLAUDE.md belongs in Future Work, not the headline. The
closing caveat ("largely vibe-coded ... at the cost of your own patience," lines
67–71) is a register/credibility leak. Separately, the "static SVG but for
monospace terminal grids" and "borrows web technology" phrasings (lines 49–52)
drift toward the browser/styling-language scope the style guide explicitly warns
against.

**Fix.** Cut the Abstract to ~2–3 tight paragraphs. Correct all four typos.
Either remove the button/clickable semantic claim or qualify it explicitly as
deferred to §13 (the "what TWP is _not_" list and the polyfill caveat both move
into §1/§3.2). De-jokify the closing caveat: "The bundled implementation is an
experimental polyfill (§3.2), not a production-ready or fully conformant
renderer; it exists for testing and demonstration." Soften "static SVG" /
"borrows web technology" toward neutral phrasing ("a declarative description the
terminal renders," "reuses web layout concepts").

### 3.3 (Major / strategic) The normative Phase-1 surface is too large to say yes to

**Location:** §7.5 (passthrough), §5.5 (svg); cross-cuts the whole doc. **Raised
by:** Adoption (highest-leverage fix), Framing (scope-drift phrasings),
Completeness (multiplexers — see 3.4).

**Problem.** The prose claims a "small, bounded vocabulary," but the §7.5
passthrough rule forwards arbitrary CSS to the layout engine, which in practice
conscripts a **full CSS engine** into conformant behavior. Combined with `svg`,
the "subset a terminal can implement" is not actually small. A single terminal
maintainer evaluating "can I implement this?" sees an unbounded surface and says
no. Compounding it: the polyfill demonstrates _everything_ through the KGP pixel
path — so the headline accessibility/selectability benefit that motivates TWP is
**literally unobservable in the shipped artifact**.

**Fix (highest-leverage change in the whole review).** Narrow the **normative**
Phase-1 ask to a static, text-exposable display block, and **demote
arbitrary-CSS passthrough and SVG to non-normative/optional**. State plainly
which small set a conformant terminal MUST implement (the typed nodes + typed
style keys), and mark passthrough + `svg` as MAY-level extensions. This makes
the core small enough that one maintainer could implement it in a weekend, which
is the precondition for adoption.

### 3.4 (Major) Terminal multiplexers (tmux/screen) are never mentioned

**Location:** Whole document (absent); polyfill is a KGP-emitting PTY proxy.
**Raised by:** Completeness (the gap a terminal maintainer raises first),
Adoption (largest real-world deployment blocker).

**Problem.** The single largest real-world deployment blocker for any new escape
sequence is multiplexer passthrough, and tmux/screen are not mentioned anywhere.
Because the polyfill is a KGP-emitting PTY proxy, the multiplexer question is
doubly acute. Its omission is the first thing a terminal maintainer will notice.

**Fix.** Add a subsection (Compatibility §10 or Future Work §13) stating the
multiplexer situation honestly: TWP rides standard APC, which multiplexers
_should_ pass through under the "unknown ⇒ ignore" model, but passthrough of APC
and of the polyfill's KGP backend through tmux/screen is a known open problem,
deferred. Candor here is worth more than a solution.

### 3.5 (Major) Two broken `§11` cross-references point at Security instead of Future Work

**Location:** §9 Coexistence (line 727) and §10 item 1 (line 738); both should
target §13. **Raised by:** Consistency (two findings), Editorial.

**Problem.** §9's coexistence paragraph defers the tighter cell-by-cell model to
"future work (§11)" and §10's wrapper-unaware note points to "§11" for the
plain-text fallback — but §11 is **Security Considerations**, which contains
neither. Both targets are §13 (the plain-text-fallback bullet there already
back-references §10, so the corrected §10↔§13 pair becomes mutually consistent).
An implementer following either reference lands on the wrong section. These are
not normative, but a spec whose own cross-references are wrong reads as
un-proofread.

**Fix.** Change both `(§11)` / `see §11` to `§13`.

### 3.6 (Major) Prior-art survey: a factual product-name error and a missing closest precedent

**Location:** §12 (lines 727, 788 for `ratatouille`); §12 omits DomTerm.
**Raised by:** Prior-art (incomplete survey + credibility error), Editorial
(`ratatouille` typo).

**Problem.** Two credibility issues in the prior-art handling. First, the
ratatui TUI framework is misnamed **"ratatouille" twice** (lines 727 and 788) —
a factual product-name error that signals the survey wasn't checked. Second, §12
**omits DomTerm**, the single closest precedent: a terminal that already renders
application-supplied declarative HTML/SVG. A reviewer who knows the space will
notice both. The survey also never states the crispest, most defensible novelty
claim — that the genuinely new idea is _which side of the wire the layout engine
lives on_ (the terminal, not the application) — and omits several precedents a
reviewer would expect (ReGIS, Arcan/letoram, Notcurses, named declarative TUI
frameworks, iTerm2's status-bar components).

**Fix.** Correct "ratatouille" → "ratatui" in both places. Add DomTerm to §12 as
the closest precedent and distinguish TWP from it (TWP is a small bounded
vocabulary tied to the terminal's grid/theme, not full HTML). State the
layout-engine-side novelty claim explicitly. Add brief mentions of the other
expected precedents so the survey reads as complete.

---

## 4. Minor issues & nits

Technical-soundness minors (real, but low impact — most are spec-clarity, not
bugs):

- **§4.3 "APC-safe by construction" vs §4.1's 8-bit C1 forms** (lines 422–424 vs
  389–396) → _Partly confirmed._ §4.3 justifies safety only via "no raw ESC,"
  but §4.1 says the 8-bit `0x9C` (ST) and `0x9F` (APC) "MAY be accepted" — and
  those bytes are common UTF-8 continuation bytes (Ü=`c3 9c`, ß=`c3 9f`,
  ş=`c5 9f`). A renderer exercising the §4.1 "MAY" and scanning for bare
  `0x9C`/`0x9F` would corrupt the framing of ordinary multilingual text.
  **Important caveat:** the reference parser only triggers on `0x1B`, so nothing
  actually breaks in the implementation today — this is a spec-clarity
  inconsistency, not a wire break. **Fix:** drop the §4.1 permission to accept
  8-bit C1 ST/APC forms (matches the parser), _or_ restate §4.3 as "APC-safe
  when terminated by the 2-byte `ESC \` form" and add "a renderer MUST NOT scan
  for bare `0x9C`/`0x9F` inside a UTF-8 payload."
- **Header/payload `;` split rule under-specified** (§4.1 lines 386, 393–395) →
  _Partly confirmed._ The spec frames the wire as `twp;<header>;<payload>` and
  says `;` "separates header from payload" but never states **split on the first
  `;` only**; JSON payloads may legitimately contain `;` (e.g. text content
  `"t":"a; b"`, multi-declaration passthrough). The parser gets it right
  (`position(|&b| b == b';')`). _Caveat: the originally-cited demo evidence
  (`box-shadow:"0 8px 22px ..."`) contains no semicolon; no shipped payload
  currently emits a `;` in the body, so this is a latent gap, not a live bug._
  **Fix:** add one sentence — "The header extends to the first `;`; everything
  after is the payload (which may itself contain `;`). The header MUST NOT
  contain a `;`."
- **`v`/`c`/`r` marked "Required: yes" but the parser tolerates all three
  missing** (§4.2 lines 405–409; `main.rs:73–101`) → _Confirmed._ `v` is
  validated only when present; `c`/`r` default to 20/4; an empty header
  `twp;;{...}` renders. The parser's own comment says `v=1` is "required by spec
  but tolerated when missing." **Fix:** state the requirement normatively with a
  defined consequence for omission (e.g. "A sender MUST include `c`/`r`; a
  renderer MAY assume a default footprint if absent; a missing `v` is treated as
  `v=1`"), and note the polyfill's leniency is a non-normative liberty. _(The
  consistency dimension's separate framing of this as a c/r-vs-no-op
  contradiction was only **partly** confirmed — the §4.3 no-op concerns the
  payload, not the header, so there is no real contradiction there; the genuine
  kernel is the missing normative consequence.)_
- **§4.3 "control characters `\u`-escaped ... by construction" relies on
  unstated UTF-8 + encoder assumptions** (lines 422–424) → _Confirmed._ No
  "payload MUST be UTF-8" appears anywhere, and the escaping is presented as
  automatic. **Fix:** add "The payload MUST be UTF-8-encoded JSON; senders MUST
  emit it compactly with all control characters U+0000–U+001F `\u`-escaped."
- **§7.3 percentage semantics don't match CSS for
  `border-radius`/`gap`/`padding`** (line 597; `render.rs:1075,1086`) →
  _Confirmed._ The table says "50% of the parent's corresponding axis," but the
  impl delegates to the CSS engine, where percentage resolution is
  property-specific (border-radius resolves against the element's own box;
  padding/gap against inline-size). **Fix:** qualify the table to "percentages
  follow CSS resolution for the property they appear on," or restrict
  percentages to width/height in Phase 1.
- **`subscale` clamps `n ≤ d` (fraction capped at 1.0); `d=0`/absent → full
  size** (§7.4 line 631; `render.rs:799–801`) → _Confirmed (nit)._ Documented as
  "drawn at `n/d` of the cell block" with no hint of the clamp or the `d=0`
  fallback. **Fix:** half-sentence — "subscale shrinks only (`n` is clamped to
  `≤ d`; `d` must be `> 0`, else the glyph renders at full block size)."

Internal-consistency / cross-check nits:

- **`protocol.rs:20–24` Node doc comment lists a stale node set**
  (box/text/$param/$<name>) → missing flex/mono/svg/img/stack that the spec
  defines and `render.rs` dispatches. _Confirmed (no behavioral impact; doc
  comment only)._ **Fix:** update the comment to the full §5 set.
- **§5 node skeleton (line 449) omits `img`/`name`/`props`** that the very next
  table documents → _Confirmed (cosmetic)._ **Fix:** add a trailing "… plus
  `img`/`name`/`props`, see table" or note the skeleton shows common fields
  only.
- **References label the Kitty text-sizing protocol "(OSC 66)"** but the
  in-document name and the cite should match → **Fix:** use one neutral cite
  label (e.g. `[KITTY-TEXT-SIZING]`) in both §12 and the reference entry.

Editorial / prose nits:

- **Status line typo:** line 3 `proposol` → `proposal`.
- **Capitalization:** "Terminal graphics protocol" (line 34) is miscapitalized;
  use the canonical "Kitty graphics protocol."
- **Hyphenation:** Abstract line 37 "user-interfaces" (noun) → "user
  interfaces."
- **US/UK spelling:** reconcile `color`/`colour` to a single convention (US
  `color`, matching the wire keyword).
- **Em-dashes:** replace spaced hyphens used as em-dashes (e.g. line 37 " - ")
  with true em-dashes.
- **§2 accessibility "do no harm" MUST is unverifiable as written** (lines
  272–285) → give it a testable shape (e.g. "a renderer MUST NOT remove
  accessibility from content it did not render") or soften to SHOULD until an
  accessibility model exists.

---

## 5. The strategic question (adoption)

**Candid verdict: as currently scoped, adoption odds are low — but the
document's honesty is a real asset, and the fix is structural, not rhetorical.**

The proposal has two genuine strategic strengths. First, it handles the "don't
bloat my terminal" objection structurally via "unknown ⇒ ignore" — a terminal
can implement a subset, and unaware terminals swallow the APC sequence. Second,
it is candid about its provisional status (Status §, §3.4 "genuinely unknown").
Preserve both; they are what make the proposal credible rather than salesy.

But three load-bearing obstacles are currently unconfronted:

1. **The normative surface is too big.** §7.5 passthrough conscripts a full CSS
   engine into conformant behavior, contradicting the "small, bounded
   vocabulary" claim. A maintainer evaluating "can I implement this?" sees an
   unbounded surface.
2. **Multiplexers are unaddressed.** tmux/screen passthrough is the single
   largest deployment blocker for any new escape sequence, and the document is
   silent.
3. **The benefit is unobservable in the shipped artifact.** The polyfill
   demonstrates everything through the KGP pixel path — the exact path TWP
   exists to transcend — so the headline accessibility/selectability win cannot
   be seen in the demo. The selling point is asserted, not shown.

**The single highest-leverage change** is the one in §3.3: **narrow the
normative Phase-1 ask to a static, text-exposable display block, and demote
arbitrary-CSS passthrough and SVG to non-normative/optional.** This shrinks the
"MUST implement" core to something a single maintainer could ship in a weekend —
which is the precondition for any "yes." It also makes the doc's own claim
("small, bounded vocabulary") true, and it makes a future native demo capable of
_showing_ the selectability/accessibility benefit on the small core rather than
asserting it. Pair it with an honest multiplexer note (§3.4) and the proposal
goes from "interesting but unbounded" to "small enough to try."

---

## 6. Prioritized action list

1. **Rewrite the Abstract** — cut to ~2–3 paragraphs, fix all four typos
   (`proposol`, `propgrams`, two `form`/`from`), remove or defer the
   button/clickable semantic claim, de-jokify the polyfill caveat, soften
   "static SVG"/"borrows web technology." (§3.2)
2. **Document the typed style keys** — add `border` (`{width, color}`,
   solid-only Phase 1, all four edges) and the typed text fields
   `font-size`/`font-weight`/`text-align` to §7/§8. (§3.1)
3. **Narrow the normative Phase-1 surface** — make the typed nodes + typed style
   keys the MUST-implement core; demote arbitrary-CSS passthrough and `svg` to
   non-normative/optional. (§3.3 — highest strategic leverage)
4. **Fix the two broken `§11` cross-references** in §9 (line 727) and §10
   (line 738) to point at `§13`. (§3.5)
5. **Correct "ratatouille" → "ratatui"** in both places (lines 727, 788), add
   DomTerm to §12 as the closest precedent, and state the layout-engine-side
   novelty claim explicitly. (§3.6)
6. **Add an honest multiplexer note** (§10 or §13) — APC should pass through
   under "unknown ⇒ ignore," but tmux/screen passthrough of APC and the
   polyfill's KGP backend is a known open problem, deferred. (§3.4)
7. **Tighten the transport-safety story** — state "payload MUST be UTF-8-encoded
   JSON, control chars `\u`-escaped," add the "split on first `;`" rule, and
   reconcile §4.1's 8-bit C1 permission with the §4.3 safety claim (drop the
   8-bit "MAY" to match the parser). (§4)
8. **Make header-field requirements normative** — give `v`/`c`/`r` "Required:
   yes" a defined consequence for omission and treat a missing `v` as `v=1`;
   note the polyfill's leniency is non-normative. (§4)
9. **Qualify §7.3 percentage semantics** to follow CSS per-property resolution
   (or restrict percentages to width/height in Phase 1). (§4)
10. **Sweep the small inconsistencies** — update the stale `protocol.rs` Node
    doc comment, annotate the §5 skeleton, document the `subscale` `n ≤ d`
    clamp, unify `color`/`colour` and em-dashes, fix the "Terminal graphics
    protocol" capitalization and the "user-interfaces" hyphenation, and either
    make the §2 accessibility MUST testable or soften it. (§4)
