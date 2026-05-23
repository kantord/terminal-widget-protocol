#!/usr/bin/env bash
# Demonstration of Terminal Widget Protocol Phase 1.
# Run inside `twp-proxy zsh` (or bash) under Kitty to see widgets render
# inline. Each TWP APC is a one-liner: `ESC _ twp ; v=1 ; c=COLS,ROWS ; <json> ESC \`

set -e

twp() {
  local hdr="$1" json="$2"
  printf '\x1b_twp;%s;%s\x1b\\' "$hdr" "$json"
}

echo "TWP Phase 1 demo"
echo "================"
echo

echo "1) Greeting card — single box with centered text:"
twp 'v=1,c=20,r=4' '{
  "S": {"n":"box","s":{"display":"flex","justify-content":"center","align-items":"center","width":400,"height":160,"background":"#1e293b","border-radius":24},
        "c":[{"n":"text","t":"Hello, TWP!","s":{"font-size":36,"color":"#ffffff","font-weight":"bold"}}]}
}'
echo
echo

echo "2) Traffic light — flex row with three coloured discs:"
twp 'v=1,c=20,r=4' '{
  "S": {"n":"box","s":{"display":"flex","flex-direction":"row","justify-content":"space-around","align-items":"center","width":400,"height":160,"background":"#2a2d3a","border-radius":40},
        "c":[
          {"n":"box","s":{"width":100,"height":100,"background":"#f04646","border-radius":"50%"}},
          {"n":"box","s":{"width":100,"height":100,"background":"#fac83c","border-radius":"50%"}},
          {"n":"box","s":{"width":100,"height":100,"background":"#50dc6e","border-radius":"50%"}}
        ]}
}'
echo
echo

echo "3) Three status tags sharing one component definition:"
twp 'v=1,c=20,r=4' '{
  "S": {"n":"box","s":{"display":"flex","flex-direction":"row","justify-content":"space-around","align-items":"center","width":400,"height":160,"background":"#0f172a"},
        "c":[
          {"n":"$tag","props":{"label":"build"}},
          {"n":"$tag","props":{"label":"test"}},
          {"n":"$tag","props":{"label":"ship"}}
        ]},
  "C": {
    "tag": {"n":"box","s":{"display":"flex","justify-content":"center","align-items":"center","width":110,"height":50,"border-radius":24,"background":"#16a34a","color":"#ffffff","font-size":22,"font-weight":"bold"},
            "c":[{"n":"$param","name":"label"}]}
  }
}'
echo
echo

echo "Done."
