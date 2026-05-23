#!/usr/bin/env bash
# Exhaustive test of the `mono` node type — each character in its own
# cell-width box, zero drift by construction.
#
# Every test has a "Native:" line (plain echo) and a "Widget:" line
# (mono node), so you can visually compare character-by-character.
#
# Hardcoded colours match Kitty config: bg=#0a1e24  fg=#ecefc1
set -e
twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }

BG='"#0a1e24"'
FG='"#ecefc1"'

mono() {
  local cols="$1" text="$2" extra="${3:-}"
  local style="\"font-size\":32,\"color\":$FG$extra"
  twp "v=1,c=$cols,r=1" "{\"S\":{\"n\":\"mono\",\"t\":\"$text\",\"s\":{$style,\"background\":$BG}}}"
}

echo "mono node — cell-grid-aligned text rendering"
echo "============================================="
echo

# ─── 1. Basic parity ─────────────────────────────────────────────────
echo "1) Basic parity — pangram"
echo "   Native:  The quick brown fox jumps over the lazy dog. 0123456789"
echo -n "   Widget:  "
mono 60 "The quick brown fox jumps over the lazy dog. 0123456789"
echo
echo

# ─── 2. Bold parity ──────────────────────────────────────────────────
echo "2) Bold parity"
echo "   Native (bold):  STATUS PASS  STATUS WARN  STATUS FAIL  STATUS BLOCKED"
echo -n "   Widget (bold):  "
twp 'v=1,c=65,r=1' '{"S":{"n":"mono","t":"STATUS PASS  STATUS WARN  STATUS FAIL  STATUS BLOCKED","s":{"font-size":32,"font-weight":"bold","color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

# ─── 3. Narrow glyphs (pipes) ────────────────────────────────────────
echo "3) Narrow glyphs — 50 pipes"
echo "   Native:  ||||||||||||||||||||||||||||||||||||||||||||||||||"
echo -n "   Widget:  "
mono 50 "||||||||||||||||||||||||||||||||||||||||||||||||||"
echo
echo

# ─── 4. Wide glyphs (Ms) ─────────────────────────────────────────────
echo "4) Wide glyphs — 50 Ms"
echo "   Native:  MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM"
echo -n "   Widget:  "
mono 50 "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM"
echo
echo

# ─── 5. Words with spaces ────────────────────────────────────────────
echo "5) Words — space-character alignment"
echo "   Native:  one two three four five six seven eight nine ten!"
echo -n "   Widget:  "
mono 50 "one two three four five six seven eight nine ten!"
echo
echo

# ─── 6. Digits only ──────────────────────────────────────────────────
echo "6) Digits — ruler test (count the columns)"
echo "   Native:  0         1         2         3         4"
echo "            0123456789012345678901234567890123456789012345678"
echo -n "   Widget:  "
mono 49 "0123456789012345678901234567890123456789012345678"
echo
echo

# ─── 7. Alternating characters ────────────────────────────────────────
echo "7) Alternating pattern — exposes uneven per-glyph centering"
echo "   Native:  iMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiM"
echo -n "   Widget:  "
mono 50 "iMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiMiM"
echo
echo

# ─── 8. Mono inside flex ─────────────────────────────────────────────
echo "8) Mono inside flex — three mono labels in a flex row"
twp 'v=1,c=30,r=2' '{
  "S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-between","align-items":"center","width":"100%","height":"100%","background":"#1e293b","padding":8},
       "c":[
         {"n":"mono","t":"BUILD","s":{"font-size":28,"color":"#86efac"}},
         {"n":"mono","t":"TEST","s":{"font-size":28,"color":"#fcd34d"}},
         {"n":"mono","t":"DEPLOY","s":{"font-size":28,"color":"#fca5a5"}}
       ]}
}'
echo
echo

# ─── 9. Mono with background (badge style) ───────────────────────────
echo "9) Mono badge — mono text with styled background"
echo -n "   "
twp 'v=1,c=8,r=2' '{"S":{"n":"mono","t":" PASS ","s":{"font-size":32,"color":"#ffffff","background":"#16a34a","border-radius":8}}}'
echo -n "  "
twp 'v=1,c=8,r=2' '{"S":{"n":"mono","t":" FAIL ","s":{"font-size":32,"color":"#ffffff","background":"#dc2626","border-radius":8}}}'
echo -n "  "
twp 'v=1,c=8,r=2' '{"S":{"n":"mono","t":" WARN ","s":{"font-size":32,"color":"#000000","background":"#fbbf24","border-radius":8}}}'
echo
echo

# ─── 10. Mono in vertical flex ────────────────────────────────────────
echo "10) Mono in vertical flex column"
twp 'v=1,c=20,r=4' '{"S":{"n":"flex","s":{"flex-direction":"column","justify-content":"space-evenly","align-items":"start","width":"100%","height":"100%","background":"#1e293b","border-radius":8,"padding":8},"c":[
  {"n":"mono","t":"line one","s":{"font-size":28,"color":"#86efac"}},
  {"n":"mono","t":"line two","s":{"font-size":28,"color":"#fcd34d"}},
  {"n":"mono","t":"line three","s":{"font-size":28,"color":"#fca5a5"}}
]}}'
echo
echo

# ─── 11. Mono in nested flex (header + body) ──────────────────────────
echo "11) Mono inside nested flex — dashboard card"
twp 'v=1,c=30,r=4' '{"S":{"n":"flex","s":{"flex-direction":"column","width":"100%","height":"100%","background":"#1e293b","border-radius":12},"c":[
  {"n":"flex","s":{"flex-direction":"row","justify-content":"space-between","align-items":"center","padding":8,"background":"#0f172a"},"c":[
    {"n":"mono","t":"METRICS","s":{"font-size":24,"color":"#ffffff","font-weight":"bold"}},
    {"n":"mono","t":"LIVE","s":{"font-size":24,"color":"#16a34a"}}
  ]},
  {"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","padding":8},"c":[
    {"n":"flex","s":{"flex-direction":"column","align-items":"center"},"c":[
      {"n":"mono","t":"99.9%","s":{"font-size":28,"color":"#86efac","font-weight":"bold"}},
      {"n":"mono","t":"uptime","s":{"font-size":20,"color":"#94a3b8"}}
    ]},
    {"n":"flex","s":{"flex-direction":"column","align-items":"center"},"c":[
      {"n":"mono","t":"42ms","s":{"font-size":28,"color":"#fcd34d","font-weight":"bold"}},
      {"n":"mono","t":"p99","s":{"font-size":20,"color":"#94a3b8"}}
    ]},
    {"n":"flex","s":{"flex-direction":"column","align-items":"center"},"c":[
      {"n":"mono","t":"1.2k","s":{"font-size":28,"color":"#7dd3fc","font-weight":"bold"}},
      {"n":"mono","t":"rps","s":{"font-size":20,"color":"#94a3b8"}}
    ]}
  ]}
]}}'
echo
echo

# ─── 12. Grid of mono cells ──────────────────────────────────────────
echo "12) Grid-like layout — mono cells in rows of flex"
twp 'v=1,c=24,r=3' '{"S":{"n":"flex","s":{"flex-direction":"column","width":"100%","height":"100%","background":"#0f172a","padding":4,"gap":4},"c":[
  {"n":"flex","s":{"flex-direction":"row","gap":4},"c":[
    {"n":"mono","t":" GET  ","s":{"font-size":22,"color":"#86efac","background":"#1e293b"}},
    {"n":"mono","t":" /api/users      ","s":{"font-size":22,"color":"#e2e8f0","background":"#1e293b"}},
    {"n":"mono","t":" 200 ","s":{"font-size":22,"color":"#86efac","background":"#14532d"}}
  ]},
  {"n":"flex","s":{"flex-direction":"row","gap":4},"c":[
    {"n":"mono","t":" POST ","s":{"font-size":22,"color":"#fcd34d","background":"#1e293b"}},
    {"n":"mono","t":" /api/auth/login  ","s":{"font-size":22,"color":"#e2e8f0","background":"#1e293b"}},
    {"n":"mono","t":" 401 ","s":{"font-size":22,"color":"#fca5a5","background":"#7f1d1d"}}
  ]},
  {"n":"flex","s":{"flex-direction":"row","gap":4},"c":[
    {"n":"mono","t":" GET  ","s":{"font-size":22,"color":"#86efac","background":"#1e293b"}},
    {"n":"mono","t":" /api/health      ","s":{"font-size":22,"color":"#e2e8f0","background":"#1e293b"}},
    {"n":"mono","t":" 200 ","s":{"font-size":22,"color":"#86efac","background":"#14532d"}}
  ]}
]}}'
echo
echo

echo "============================================="
echo "If characters in the Widget lines land in the same columns as the"
echo "Native lines, cell-grid alignment is working. Remaining differences"
echo "are glyph-shape differences between our renderer and Kitty's."
