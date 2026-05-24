#!/usr/bin/env bash
# Automated visual comparison: Kitty native vs TWP mono rendering.
#
# For each test case:
#   1. Renders text in a headless Kitty (via Xvfb) and screenshots it
#   2. Renders the same text as a TWP mono widget (PNG extraction)
#   3. Compares per-cell glyph alignment using center-of-mass analysis
#
# Dependencies: xvfb-run, kitty, import (ImageMagick), python3 + Pillow + numpy
# Run from the twp-proxy directory.

set -e

BINARY="$(pwd)/target/release/twp-proxy"
RESULTS="/tmp/twp-visual-test"
rm -rf "$RESULTS" && mkdir -p "$RESULTS"

# Use the user's Kitty font for fair comparison
FONT=$(grep "^font_family" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_family\s*//')
FSIZE=$(grep "^font_size" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_size\s*//')
: "${FONT:=monospace}"
: "${FSIZE:=16}"

echo "TWP Visual Comparison Test"
echo "=========================="
echo "Font: $FONT @ ${FSIZE}pt"
echo

# ── Helpers ─────────────────────────────────────────────────────────

screenshot_kitty() {
    local text="$1" name="$2" w="$3" h="$4"
    timeout 12 xvfb-run -s "-screen 0 ${w}x${h}x24" bash -c "
        kitty --config=NONE \
              --override='font_family=$FONT' \
              --override='font_size=$FSIZE' \
              --override='background=#0a1e24' \
              --override='foreground=#ecefc1' \
              --override='remember_window_size=no' \
              --override='initial_window_width=$w' \
              --override='initial_window_height=$h' \
              --override='window_padding_width=0' \
              --override='confirm_os_window_close=0' \
              bash -c 'printf \"%s\" \"$text\"; sleep 3' &
        sleep 2
        import -window root '$RESULTS/kitty_${name}.png' 2>/dev/null
        kill %1 2>/dev/null; wait 2>/dev/null
    " 2>/dev/null
}

render_twp() {
    local text="$1" name="$2" cols="$3" rows="$4" extra="$5"
    local style="\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"${extra:+,$extra}"
    local json="{\"S\":{\"n\":\"mono\",\"t\":\"$text\",\"s\":{$style}}}"

    # Write a temp script that emits the APC — piping raw ESC bytes
    # directly into bash doesn't work (bash tries to execute them).
    local script="$RESULTS/emit_${name}.sh"
    cat > "$script" <<SEOF
#!/bin/bash
printf '\x1b_twp;v=1,c=$cols,r=$rows;$json\x1b\\\\'
exit
SEOF
    chmod +x "$script"
    KITTY_WINDOW_ID=fake "$BINARY" bash "$script" > "$RESULTS/raw_${name}.bin" 2>/dev/null

    python3 -c "
import re, base64
data = open('$RESULTS/raw_${name}.bin', 'rb').read()
pat = re.compile(rb'\x1b_G([^;]+);(.*?)\x1b\\\\', re.DOTALL)
chunks = []
for h, p in pat.findall(data):
    kv = dict(x.split(b'=',1) for x in h.split(b',') if b'=' in x)
    if b'i' in kv: chunks = [p]
    else: chunks.append(p)
    if kv.get(b'm') != b'1' and chunks:
        open('$RESULTS/twp_${name}.png','wb').write(base64.b64decode(b''.join(chunks)))
        break
"
}

compare() {
    local name="$1" cols="$2"
    python3 -c "
from PIL import Image
import numpy as np

try:
    twp = np.array(Image.open('$RESULTS/twp_${name}.png').convert('L'))
    kitty = np.array(Image.open('$RESULTS/kitty_${name}.png').convert('L'))
except Exception as e:
    print(f'SKIP ({e})')
    exit(0)

cols = $cols

# Crop kitty to same height as twp (kitty screenshot has extra rows)
if kitty.shape[0] > twp.shape[0]:
    kitty = kitty[:twp.shape[0], :]

def cell_centers(img, ncols):
    cell_w = img.shape[1] // ncols if ncols > 0 else 1
    centers = []
    for i in range(ncols):
        x0, x1 = i * cell_w, min((i+1) * cell_w, img.shape[1])
        ink = img[:, x0:x1] < 180
        if ink.any():
            centers.append(float(np.where(ink)[1].mean()) / cell_w)
        else:
            centers.append(None)
    return centers

tc = cell_centers(twp, cols)
kc = cell_centers(kitty, cols)

drifts = [abs(t - k) for t, k in zip(tc, kc) if t is not None and k is not None]
if not drifts:
    print('SKIP (no comparable cells)')
    exit(0)

max_d = max(drifts)
avg_d = sum(drifts) / len(drifts)
# Pass if center-of-mass within 25% of cell width
status = 'PASS' if max_d < 0.25 else 'FAIL'
print(f'{status} max_drift={max_d:.3f} avg_drift={avg_d:.3f} ({len(drifts)} cells)')

# Save side-by-side
try:
    t = Image.open('$RESULTS/twp_${name}.png')
    k = Image.open('$RESULTS/kitty_${name}.png')
    h = max(t.height, k.height)
    combined = Image.new('RGB', (max(t.width, k.width), h * 2 + 4), (80, 80, 80))
    combined.paste(t, (0, 0))
    combined.paste(k.crop((0, 0, min(k.width, t.width), min(k.height, t.height))), (0, t.height + 4))
    combined.save('$RESULTS/diff_${name}.png')
except: pass
"
}

# ── Test cases ──────────────────────────────────────────────────────

pass=0; fail=0; skip=0

run_test() {
    local name="$1" text="$2" cols="$3" rows="$4" extra="$5"
    echo -n "  $name: "

    render_twp "$text" "$name" "$cols" "$rows" "$extra"

    if [ ! -f "$RESULTS/twp_${name}.png" ]; then
        echo "SKIP (no TWP PNG)"; skip=$((skip+1)); return
    fi

    # Get TWP image size for Kitty window
    local dims
    dims=$(python3 -c "from PIL import Image; i=Image.open('$RESULTS/twp_${name}.png'); print(i.width, i.height)")
    local w h
    w=$(echo "$dims" | cut -d' ' -f1)
    h=$(echo "$dims" | cut -d' ' -f2)

    screenshot_kitty "$text" "$name" "$((w + 10))" "$((h + 10))"

    if [ ! -f "$RESULTS/kitty_${name}.png" ]; then
        echo "SKIP (screenshot failed)"; skip=$((skip+1)); return
    fi

    local result
    result=$(compare "$name" "$cols")
    echo "$result"

    case "$result" in
        PASS*) pass=$((pass+1)) ;;
        FAIL*) fail=$((fail+1)) ;;
        *)     skip=$((skip+1)) ;;
    esac
}

run_test "letters"   "ABCDEFGHIJ"           10 1
run_test "pangram"   "The quick brown fox"   19 1
run_test "digits"    "0123456789012345"       16 1
run_test "pipes"     "||||||||||||||||||||"   20 1
run_test "mixed"     "Hello, world! 123"     17 1
run_test "wide"      "MMMMMMMMMMMMMMMMMMMM"  20 1
run_test "spaces"    "A B C D E F G H I J"   19 1

total=$((pass + fail + skip))
echo
echo "=========================="
echo "Results: $pass passed, $fail failed, $skip skipped (of $total)"
echo "Diffs:   $RESULTS/diff_*.png"
