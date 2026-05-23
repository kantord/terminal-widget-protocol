#!/usr/bin/env bash
# Test the component system — $defs, $param substitution, reuse,
# nesting, and graceful degradation on missing components.
set -e
twp() { printf '\x1b_twp;%s;%s\x1b\\' "$1" "$2"; }

echo "Component system — test cases"
echo "=============================="
echo

# ─── 1. Basic component ──────────────────────────────────────────────
echo "1) Single component def + invocation"
twp 'v=1,c=12,r=2' '{
  "S":{"n":"$pill","props":{"label":"hello"}},
  "C":{"pill":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#7c3aed","border-radius":20,"color":"#ffffff","font-size":20},
               "c":[{"n":"$param","name":"label"}]}}
}'
echo
echo

# ─── 2. One def, many invocations ─────────────────────────────────────
echo "2) One def, four invocations"
twp 'v=1,c=24,r=2' '{
  "S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#0f172a"},
       "c":[
         {"n":"$tag","props":{"label":"build"}},
         {"n":"$tag","props":{"label":"test"}},
         {"n":"$tag","props":{"label":"ship"}},
         {"n":"$tag","props":{"label":"merge"}}
       ]},
  "C":{"tag":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":80,"height":32,"border-radius":16,"background":"#16a34a","color":"#ffffff","font-size":14,"font-weight":"bold"},
              "c":[{"n":"$param","name":"label"}]}}
}'
echo
echo

# ─── 3. Prop is a full node tree ──────────────────────────────────────
echo "3) Prop value as a node tree (not just a string)"
twp 'v=1,c=20,r=2' '{
  "S":{"n":"$card","props":{"content":{"n":"flex","s":{"flex-direction":"row","gap":8,"align-items":"center"},"c":[
    {"n":"box","s":{"width":24,"height":24,"background":"#f04646","border-radius":"50%"}},
    {"n":"text","t":"Alert!","s":{"font-size":16,"color":"#fca5a5"}}
  ]}}},
  "C":{"card":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":8,"padding":8},
               "c":[{"n":"$param","name":"content"}]}}
}'
echo
echo

# ─── 4. Missing component — graceful degradation ─────────────────────
echo "4) Missing component (should silently render empty box)"
twp 'v=1,c=10,r=2' '{"S":{"n":"$does_not_exist","props":{}}}'
echo "(if you see an empty area above, degradation is working)"
echo

# ─── 5. Nested components ────────────────────────────────────────────
echo "5) Nested components (outer wraps inner)"
twp 'v=1,c=20,r=3' '{
  "S":{"n":"$frame","props":{"body":{"n":"$badge","props":{"label":"OK"}}}},
  "C":{
    "frame":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#0f172a","border-radius":12,"padding":12},
             "c":[{"n":"$param","name":"body"}]},
    "badge":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":100,"height":40,"background":"#16a34a","border-radius":20,"color":"#ffffff","font-size":20,"font-weight":"bold"},
             "c":[{"n":"$param","name":"label"}]}
  }
}'
echo
echo

# ─── 6. Unfilled param ───────────────────────────────────────────────
echo "6) Unfilled param (label not passed — should show empty box)"
twp 'v=1,c=10,r=2' '{
  "S":{"n":"$pill","props":{}},
  "C":{"pill":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#7c3aed","border-radius":20},
               "c":[{"n":"$param","name":"label"}]}}
}'
echo "(purple pill with empty interior = unfilled param placeholder)"
echo

echo "=============================="
echo "Each test exercises a specific component-system feature. Missing"
echo "components and unfilled params should degrade silently (empty boxes)."
