#!/usr/bin/env bash
# Phase 1 demo — exercises every flex layout mode plus the box and text
# primitives, with widgets placed alongside terminal text so the cell-grid
# alignment is visually verifiable.
#
# Run inside `twp-proxy zsh` (or bash) on a Kitty-compatible terminal.

set -e

twp() {
  local hdr="$1" json="$2"
  printf '\x1b_twp;%s;%s\x1b\\' "$hdr" "$json"
}

echo "TWP Phase 1 — flex layout showcase"
echo "==================================="
echo

# ─── 1. Widgets inline with terminal text ──────────────────────────────
echo "1) Widgets inline with terminal text (r=1 widgets share the cursor line)"
echo "   --------------------------------------------------------------------"
echo
echo -n "   Status: "
twp 'v=1,c=2,r=1' '{"S":{"n":"box","s":{"width":"100%","height":"100%","background":"#16a34a","border-radius":"50%"}}}'
echo "  online"
echo
echo -n "   Branch: "
twp 'v=1,c=8,r=1' '{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":4},"c":[{"n":"text","t":"main","s":{"font-size":14,"color":"#7dd3fc","font-weight":"bold"}}]}}'
echo "  · 12 commits ahead"
echo
echo -n "   Build:  "
twp 'v=1,c=6,r=1' '{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#16a34a","border-radius":4},"c":[{"n":"text","t":"PASS","s":{"font-size":14,"color":"#ffffff","font-weight":"bold"}}]}}'
echo "  in 12.3s"
echo

# ─── 2. Every justify-content mode side-by-side ───────────────────────
echo "2) Every justify-content mode (three dots in a c=20 row)"
echo "   -----------------------------------------------------"
echo "   The first dot's left edge should sit at a cell boundary;"
echo "   the third dot's right edge should too."
echo
for jc in start end center space-between space-around space-evenly; do
  printf "   %s\n" "$jc:"
  twp 'v=1,c=20,r=2' "{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"$jc\",\"align-items\":\"center\",\"width\":\"100%\",\"height\":\"100%\",\"background\":\"#1e293b\",\"border-radius\":8,\"padding\":8},\"c\":[{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#f04646\",\"border-radius\":\"50%\"}},{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#fac83c\",\"border-radius\":\"50%\"}},{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#50dc6e\",\"border-radius\":\"50%\"}}]}}"
  echo
done
echo

# ─── 3. flex-direction: column ────────────────────────────────────────
echo "3) flex-direction: column (vertical stack)"
echo "   ---------------------------------------"
twp 'v=1,c=14,r=4' '{"S":{"n":"flex","s":{"flex-direction":"column","justify-content":"space-evenly","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":8,"padding":6},"c":[
  {"n":"text","t":"top","s":{"font-size":14,"color":"#fca5a5"}},
  {"n":"text","t":"middle","s":{"font-size":14,"color":"#fcd34d"}},
  {"n":"text","t":"bottom","s":{"font-size":14,"color":"#86efac"}}
]}}'
echo
echo

# ─── 4. Nested flex (row + column composition) ────────────────────────
echo "4) Nested flex (header row + body column)"
echo "   --------------------------------------"
twp 'v=1,c=24,r=4' '{"S":{"n":"flex","s":{"flex-direction":"column","width":"100%","height":"100%","background":"#1e293b","border-radius":12},"c":[
  {"n":"flex","s":{"flex-direction":"row","justify-content":"space-between","align-items":"center","padding":10,"background":"#0f172a"},"c":[
    {"n":"text","t":"Deployment","s":{"font-size":16,"color":"#ffffff","font-weight":"bold"}},
    {"n":"text","t":"v2.1.0","s":{"font-size":14,"color":"#94a3b8"}}
  ]},
  {"n":"flex","s":{"flex-direction":"column","justify-content":"center","padding":10,"color":"#cbd5e1","font-size":14},"c":[
    {"n":"text","t":"production"},
    {"n":"text","t":"deployed 2 min ago"}
  ]}
]}}'
echo
echo

# ─── 5. Reusable component (one def, four invocations) ────────────────
echo "5) Reusable component — \$tag defined once, invoked four times"
echo "   -----------------------------------------------------------"
twp 'v=1,c=24,r=2' '{
  "S": {"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},
        "c":[
          {"n":"$tag","props":{"label":"build"}},
          {"n":"$tag","props":{"label":"test"}},
          {"n":"$tag","props":{"label":"ship"}},
          {"n":"$tag","props":{"label":"merge"}}
        ]},
  "C": { "tag": {"n":"flex","s":{"justify-content":"center","align-items":"center","width":80,"height":32,"border-radius":16,"background":"#16a34a","color":"#ffffff","font-size":14,"font-weight":"bold"},
                 "c":[{"n":"$param","name":"label"}]} }
}'
echo
echo

# ─── 6. box primitive — no layout, just a styled leaf ────────────────
echo "6) box primitive (no layout) — divider line"
echo "   -----------------------------------------"
echo "   Above the line"
twp 'v=1,c=24,r=1' '{"S":{"n":"box","s":{"width":"100%","height":"40%","background":"#475569"}}}'
echo
echo "   Below the line"
echo

# ─── 7. Text rendering parity — TWP text vs native terminal text ──────
echo "7) Text parity: native echo vs TWP text node (same string)"
echo "   --------------------------------------------------------"
echo "   If our font discovery + size calibration is right, the two lines"
echo "   below should look indistinguishable — same font family, same size."
echo
echo "   Native:  Hello, world! 0123456789"
echo -n "   Widget:  "
twp 'v=1,c=28,r=1' '{"S":{"n":"text","t":"Hello, world! 0123456789","s":{"font-size":32}}}'
echo
echo
echo "   Native (bold):  STATUS: OK"
echo -n "   Widget (bold):  "
twp 'v=1,c=14,r=1' '{"S":{"n":"text","t":"STATUS: OK","s":{"font-size":32,"font-weight":"bold"}}}'
echo
echo

# ─── 8. Flex vs printf space-padding ──────────────────────────────────
echo "8) Flex justify-content vs equivalent printf with spaces"
echo "   ------------------------------------------------------"
echo "   Native rows use plain text spacing. Widget rows ask flex to do"
echo "   the same job. Where the dots don't land in the same columns,"
echo "   that's the sub-cell-offset issue the Phase 2 'snap' parameter"
echo "   exists to solve."
echo
echo "   space-between (3 dots in 20 cells):"
echo "   Native: *                                       *                                       *"
echo -n "   Widget: "
twp 'v=1,c=20,r=1' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-between","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},"c":[
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}}
]}}'
echo
echo
echo "   space-around:"
echo "   Native:        *                   *                   *"
echo -n "   Widget: "
twp 'v=1,c=20,r=1' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},"c":[
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}}
]}}'
echo
echo
echo "   center (3 dots tightly grouped in middle):"
echo "   Native:                  * * *"
echo -n "   Widget: "
twp 'v=1,c=20,r=1' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"center","align-items":"center","gap":8,"width":"100%","height":"100%","background":"#0f172a"},"c":[
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}}
]}}'
echo
echo

echo "==================================="
echo "Notes:"
echo "  · Widget boundaries always cell-align (handled by Unicode placeholders)."
echo "  · Sections 7-8 are the real test for font and layout parity. Where"
echo "    you can see differences between native and widget rows, that's"
echo "    where Phase 2 work (snap parameter, font-size calibration) will go."
