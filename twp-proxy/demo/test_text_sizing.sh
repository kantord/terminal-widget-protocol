#!/usr/bin/env bash
# Side-by-side comparison: Kitty's native text-sizing protocol (OSC 66)
# vs TWP's mono node with equivalent parameters.
#
# Each test puts Kitty and TWP output on SEPARATE lines (no mid-line
# multi-row widgets) so the comparison is clean.
#
# Requires Kitty with text-sizing support.
set -e

twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }
osc66() { printf '\x1b]66;%s;%s\x07' "$1" "$2"; }

echo "Text-sizing: Kitty OSC 66 vs TWP mono node"
echo "============================================"
echo

# ─── 1. Normal (scale=1) ─────────────────────────────────────────────
echo "1) Normal text (scale=1, baseline)"
echo "   Kitty:"
echo -n "   "
osc66 "s=1" "ABCDEFGHIJ"
echo
echo "   TWP:"
echo -n "   "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDEFGHIJ","s":{"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 2. Scale=2 (2×2 cell block per char) ────────────────────────────
echo "2) Scale=2 (each char in a 2×2 cell block)"
echo "   Kitty:"
osc66 "s=2" "ABCDE"
echo
echo "   TWP:"
twp 'v=1,c=10,r=2' '{"S":{"n":"mono","t":"ABCDE","s":{"scale":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 3. Scale=3 (3×3 cell block per char) ────────────────────────────
echo "3) Scale=3 (each char in a 3×3 cell block)"
echo "   Kitty:"
osc66 "s=3" "ABC"
echo
echo "   TWP:"
twp 'v=1,c=9,r=3' '{"S":{"n":"mono","t":"ABC","s":{"scale":3,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 4. Width=2 (double-wide, single-height) ─────────────────────────
echo "4) char-width=2 (double-wide, single-height)"
echo "   Kitty:"
echo -n "   "
osc66 "w=2" "ABCDE"
echo
echo "   TWP:"
echo -n "   "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDE","s":{"char-width":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 5. Subscale n=1,d=2 (half-size glyph in normal cell) ───────────
echo "5) Subscale 1/2 (half-size glyph in normal cell)"
echo "   Kitty:"
echo -n "   "
osc66 "n=1:d=2" "ABCDEFGHIJ"
echo
echo "   TWP:"
echo -n "   "
twp 'v=1,c=10,r=1' '{"S":{"n":"mono","t":"ABCDEFGHIJ","s":{"subscale-n":1,"subscale-d":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 6. Scale=2 + subscale 1/2 (big box, normal glyph) ──────────────
echo "6) Scale=2 + subscale 1/2 (2×2 cell box, normal-size glyph)"
echo "   Kitty:"
osc66 "s=2:n=1:d=2" "ABCDE"
echo
echo "   TWP:"
twp 'v=1,c=10,r=2' '{"S":{"n":"mono","t":"ABCDE","s":{"scale":2,"subscale-n":1,"subscale-d":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 7. Long string at scale=2 (accumulated alignment) ───────────────
echo "7) Long string at scale=2 (tests accumulated alignment)"
echo "   Kitty:"
osc66 "s=2" "0123456789"
echo
echo "   TWP:"
twp 'v=1,c=20,r=2' '{"S":{"n":"mono","t":"0123456789","s":{"scale":2,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 8. Ruler comparison at scale=1 ──────────────────────────────────
echo "8) Ruler — 20 chars at scale=1 (alignment accumulation test)"
echo "   Kitty:"
echo -n "   "
osc66 "s=1" "01234567890123456789"
echo
echo "   TWP:"
echo -n "   "
twp 'v=1,c=20,r=1' '{"S":{"n":"mono","t":"01234567890123456789","s":{"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

echo "============================================"
echo "Compare each Kitty/TWP pair:"
echo "  · Same cell footprint? (chars occupy identical columns)"
echo "  · Subscale glyphs visibly smaller?"
echo "  · Scale=2 chars visibly larger and filling 2×2 blocks?"
echo
echo "Layout should be identical (same cell model). Glyph shape"
echo "will differ (different rasterizers)."
