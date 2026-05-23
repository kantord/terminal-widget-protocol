#!/usr/bin/env bash
# Exhaustive test of flex layout — every justify-content mode, both
# directions, nesting, and alignment comparison with shell spacing.
set -e
twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }

echo "flex layout — exhaustive test"
echo "============================="
echo

# ─── 1. Every justify-content mode ───────────────────────────────────
echo "1) justify-content modes (3 dots in c=20 row)"
echo
for jc in start end center space-between space-around space-evenly; do
  printf "   %s\n" "$jc:"
  twp 'v=1,c=20,r=2' "{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"$jc\",\"align-items\":\"center\",\"width\":\"100%\",\"height\":\"100%\",\"background\":\"#1e293b\",\"border-radius\":8,\"padding\":8},\"c\":[{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#f04646\",\"border-radius\":\"50%\"}},{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#fac83c\",\"border-radius\":\"50%\"}},{\"n\":\"box\",\"s\":{\"width\":36,\"height\":36,\"background\":\"#50dc6e\",\"border-radius\":\"50%\"}}]}}"
  echo
done
echo

# ─── 2. flex-direction: column ────────────────────────────────────────
echo "2) flex-direction: column"
twp 'v=1,c=14,r=4' '{"S":{"n":"flex","s":{"flex-direction":"column","justify-content":"space-evenly","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":8,"padding":6},"c":[
  {"n":"text","t":"top","s":{"font-size":14,"color":"#fca5a5"}},
  {"n":"text","t":"middle","s":{"font-size":14,"color":"#fcd34d"}},
  {"n":"text","t":"bottom","s":{"font-size":14,"color":"#86efac"}}
]}}'
echo
echo

# ─── 3. Nested flex ──────────────────────────────────────────────────
echo "3) Nested flex (header row + body column)"
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

# ─── 4. Flex vs printf spacing ────────────────────────────────────────
echo "4) Flex vs printf — space-between"
echo "   Native: *                                       *                                       *"
echo -n "   Widget: "
twp 'v=1,c=20,r=1' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-between","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},"c":[
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}}
]}}'
echo
echo
echo "   Flex vs printf — center"
echo "   Native:                  * * *"
echo -n "   Widget: "
twp 'v=1,c=20,r=1' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"center","align-items":"center","gap":8,"width":"100%","height":"100%","background":"#0f172a"},"c":[
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}},
  {"n":"box","s":{"width":12,"height":12,"background":"#ffffff","border-radius":"50%"}}
]}}'
echo
echo

# ─── 5. Inline widgets with text ──────────────────────────────────────
echo "5) r=1 widgets inline with terminal text"
echo
echo -n "   Status: "
twp 'v=1,c=2,r=1' '{"S":{"n":"box","s":{"width":"100%","height":"100%","background":"#16a34a","border-radius":"50%"}}}'
echo "  online"
echo -n "   Branch: "
twp 'v=1,c=8,r=1' '{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":4},"c":[{"n":"text","t":"main","s":{"font-size":14,"color":"#7dd3fc","font-weight":"bold"}}]}}'
echo "  · 12 commits ahead"
echo -n "   Build:  "
twp 'v=1,c=6,r=1' '{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#16a34a","border-radius":4},"c":[{"n":"text","t":"PASS","s":{"font-size":14,"color":"#ffffff","font-weight":"bold"}}]}}'
echo "  in 12.3s"
echo

echo "============================="
echo "Sections 1+4 show whether flex-computed positions fall on cell boundaries."
echo "The Phase 2 'snap' parameter will fix any sub-cell offsets."
