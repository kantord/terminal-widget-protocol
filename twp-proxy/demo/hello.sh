#!/usr/bin/env bash
# Quick TWP showcase — one example per feature. Run the test_*.sh files
# for exhaustive coverage of each area.
set -e
twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }

echo "TWP Phase 1 — quick demo"
echo "========================"
echo

echo -n "Inline status dot: "
twp 'v=1,c=2,r=1' '{"S":{"n":"box","s":{"width":"100%","height":"100%","background":"#16a34a","border-radius":"50%"}}}'
echo " online"
echo

echo "Traffic light (flex row):"
twp 'v=1,c=20,r=2' '{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#2a2d3a","border-radius":8},"c":[
  {"n":"box","s":{"width":36,"height":36,"background":"#f04646","border-radius":"50%"}},
  {"n":"box","s":{"width":36,"height":36,"background":"#fac83c","border-radius":"50%"}},
  {"n":"box","s":{"width":36,"height":36,"background":"#50dc6e","border-radius":"50%"}}
]}}'
echo

echo "Deployment card (nested flex):"
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

echo "Component reuse (\$tag × 4):"
twp 'v=1,c=24,r=2' '{
  "S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},
       "c":[{"n":"$tag","props":{"label":"build"}},{"n":"$tag","props":{"label":"test"}},{"n":"$tag","props":{"label":"ship"}},{"n":"$tag","props":{"label":"merge"}}]},
  "C":{"tag":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":80,"height":32,"border-radius":16,"background":"#16a34a","color":"#ffffff","font-size":14,"font-weight":"bold"},
              "c":[{"n":"$param","name":"label"}]}}
}'
echo

echo "Mono text (cell-grid-aligned):"
echo "   Native:  The quick brown fox jumps over the lazy dog."
echo -n "   Widget:  "
twp 'v=1,c=50,r=1' '{"S":{"n":"mono","t":"The quick brown fox jumps over the lazy dog.","s":{"font-size":32,"color":"#ecefc1","background":"#0a1e24"}}}'
echo
echo

echo "Done. See demo/test_mono.sh, test_flex.sh, test_components.sh for"
echo "exhaustive tests of each feature."
