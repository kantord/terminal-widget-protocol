#!/usr/bin/env bash
# Side-by-side comparison: Kitty's native text-sizing protocol (OSC 66)
# vs TWP's mono node with equivalent parameters.
#
# For each sizing mode, a "Kitty:" line renders text natively via OSC 66,
# and a "TWP:" line renders the same text through our mono node. If TWP's
# cell model matches Kitty's, the two lines should occupy the same cells
# and the glyphs should align vertically.
#
# Requires Kitty with text-sizing support (recent versions).
set -e

twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }

# Kitty OSC 66 helper: \e]66;<metadata>;<text>\a
osc66() { printf '\x1b]66;%s;%s\x07' "$1" "$2"; }

echo "Text-sizing: Kitty OSC 66 vs TWP mono node"
echo "============================================"
echo

# ─── 1. Normal (scale=1, default) ────────────────────────────────────
echo "1) Normal text (scale=1, baseline)"
echo -n "   Kitty:  "
osc66 "s=1" "ABCDEFGHIJ"
echo
echo -n "   TWP:    "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDEFGHIJ","s":{"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 2. Scale=2 (each char occupies 2×2 cells) ──────────────────────
echo "2) Scale=2 (each char in a 2×2 cell block)"
echo -n "   Kitty:  "
osc66 "s=2" "ABCDE"
echo
echo -n "   TWP:    "
twp 'v=1,c=10,r=2' '{"S":{"n":"mono","t":"ABCDE","s":{"scale":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 3. Scale=3 (each char occupies 3×3 cells) ──────────────────────
echo "3) Scale=3 (each char in a 3×3 cell block)"
echo -n "   Kitty:  "
osc66 "s=3" "ABC"
echo
echo -n "   TWP:    "
twp 'v=1,c=9,r=3' '{"S":{"n":"mono","t":"ABC","s":{"scale":3,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 4. Width=2 (each char occupies 2 cells wide, 1 tall) ───────────
echo "4) char-width=2 (double-wide, single-height)"
echo -n "   Kitty:  "
osc66 "w=2" "ABCDE"
echo
echo -n "   TWP:    "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDE","s":{"char-width":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 5. Fractional: subscale n=1,d=2 (half-size glyph in full cell) ─
echo "5) Subscale n=1,d=2 (half-size glyph in a normal cell)"
echo -n "   Kitty:  "
osc66 "n=1:d=2" "ABCDEFGHIJ"
echo
echo -n "   TWP:    "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDEFGHIJ","s":{"subscale-n":1,"subscale-d":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 6. Scale=2 + subscale n=1,d=2 (big box, normal glyph) ──────────
echo "6) Scale=2 + subscale 1/2 (2×2 cell box, normal-size glyph)"
echo -n "   Kitty:  "
osc66 "s=2:n=1:d=2" "ABCDE"
echo
echo -n "   TWP:    "
twp 'v=1,c=10,r=2' '{"S":{"n":"mono","t":"ABCDE","s":{"scale":2,"subscale-n":1,"subscale-d":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 7. Mixed: normal + scaled on same line ──────────────────────────
echo "7) Mixed sizing on one line: normal text, then scale=2 highlight"
echo -n "   Kitty:  hello "
osc66 "s=2" "BIG"
echo " world"
echo -n "   TWP:    hello "
twp 'v=1,c=6,r=2' '{"S":{"n":"mono","t":"BIG","s":{"scale":2,"color":"#fcd34d","background":"#0a1e24"}}}'
echo " world"
echo
echo

# ─── 8. Long string at scale=2 ───────────────────────────────────────
echo "8) Long string at scale=2 (tests accumulated alignment)"
echo -n "   Kitty:  "
osc66 "s=2" "0123456789"
echo
echo -n "   TWP:    "
twp 'v=1,c=20,r=2' '{"S":{"n":"mono","t":"0123456789","s":{"scale":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

echo "============================================"
echo "Compare each Kitty/TWP pair:"
echo "  · Same cell footprint? (chars start/end in same columns)"
echo "  · Same glyph size? (subscale halves should look half-height)"
echo "  · Vertical alignment match? (scaled chars should center in their block)"
echo
echo "Glyph shape will differ (different rasterizers), but the LAYOUT"
echo "should be identical because both use the same integer-cell model."
