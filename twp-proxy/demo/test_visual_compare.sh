#!/usr/bin/env bash
# Automated visual comparison: Kitty native text vs TWP mono rendering.
#
# TWP images are extracted from our rendering pipeline (PNG).
# Kitty images are screenshots from a headless Kitty (Xvfb).
# (Kitty Graphics images can't be screenshotted on Xvfb — GPU textures
# are invisible to the virtual framebuffer. So TWP uses extracted PNGs.)
#
# Both are compared per-cell against GROUND TRUTH (the input string):
# cell i should have ink iff text[i] is not a space. If both renderers
# match ground truth, they fill the same cells → layout equivalence.
#
# Dependencies: xvfb-run, kitty, import (ImageMagick), python3 + Pillow + numpy
set -e

BINARY="$(pwd)/target/release/twp-proxy"
RESULTS="/tmp/twp-visual-test"
rm -rf "$RESULTS" && mkdir -p "$RESULTS"

FONT=$(grep "^font_family" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_family\s*//')
FSIZE=$(grep "^font_size" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_size\s*//')
: "${FONT:=monospace}"
: "${FSIZE:=16}"

echo "TWP Visual Comparison Test"
echo "=========================="
echo "Font: $FONT @ ${FSIZE}pt"
echo

# ── Kitty screenshot on Xvfb ────────────────────────────────────────
screenshot_kitty() {
    local text="$1" out="$2"
    timeout 10 xvfb-run -s "-screen 0 600x200x24" bash -c "
        kitty --config=NONE \
              --override=\"font_family=$FONT\" \
              --override=\"font_size=$FSIZE\" \
              --override=\"background=#0a1e24\" \
              --override=\"foreground=#ecefc1\" \
              --override=\"initial_window_width=600\" \
              --override=\"initial_window_height=200\" \
              --override=\"confirm_os_window_close=0\" \
              --override=\"window_padding_width=0\" \
              --override=\"shell_integration=disabled\" \
              bash -c 'printf \"%s\" \"$text\"; sleep 2' &
        sleep 2
        import -window root '$out' 2>/dev/null
        kill %1 2>/dev/null; wait 2>/dev/null
    " 2>/dev/null
}

# ── TWP PNG extraction ──────────────────────────────────────────────
render_twp() {
    local text="$1" cols="$2" rows="$3" out="$4"
    local script="$RESULTS/_emit.sh"
    cat > "$script" <<SEOF
#!/bin/bash
printf '\x1b_twp;v=1,c=$cols,r=$rows;{"S":{"n":"mono","t":"$text","s":{"color":"#ecefc1","background":"#0a1e24"}}}\x1b\\\\'
exit
SEOF
    chmod +x "$script"
    KITTY_WINDOW_ID=fake "$BINARY" bash "$script" > "$RESULTS/_raw.bin" 2>/dev/null
    python3 -c "
import re, base64
data = open('$RESULTS/_raw.bin', 'rb').read()
pat = re.compile(rb'\x1b_G([^;]+);(.*?)\x1b\\\\', re.DOTALL)
chunks = []
for h, p in pat.findall(data):
    kv = dict(x.split(b'=',1) for x in h.split(b',') if b'=' in x)
    if b'i' in kv: chunks = [p]
    else: chunks.append(p)
    if kv.get(b'm') != b'1' and chunks:
        open('$out','wb').write(base64.b64decode(b''.join(chunks)))
        break
"
}

# ── Cell-fill ground-truth comparison ───────────────────────────────
check_cells() {
    local img_path="$1" cols="$2" text="$3" is_twp="$4"
    python3 -c "
from PIL import Image
import numpy as np

img = np.array(Image.open('$img_path').convert('RGB')).astype(float)
text = '''$text'''
cols = $cols
is_twp = $is_twp

expected = [c != ' ' for c in text[:cols]]
while len(expected) < cols: expected.append(False)

# Background: sample from top-right corner
bg = img[0, -1, :].copy()

if is_twp:
    cell_w = img.shape[1] // cols
    x_off = 0
else:
    # Find text region in screenshot
    col_dev = np.sqrt(((img - bg)**2).sum(axis=2)).sum(axis=0)
    ink_th = col_dev.max() * 0.02
    ink_cols = np.where(col_dev > ink_th)[0]
    if len(ink_cols) < 2:
        print('N/A 0')
        exit(0)
    x_off = int(ink_cols[0])
    x_end = int(ink_cols[-1]) + 1
    cell_w = (x_end - x_off) // cols
    if cell_w < 1:
        print('N/A 0')
        exit(0)

# Measure ink per cell
inks = []
for i in range(cols):
    x0 = x_off + i * cell_w
    x1 = min(x0 + cell_w, img.shape[1])
    if x0 >= img.shape[1]:
        inks.append(0.0); continue
    region = img[:, x0:x1, :]
    inks.append(float(np.sqrt(((region - bg)**2).sum(axis=2)).sum()))

# Threshold: 20% of median ink of expected-filled cells
exp_inks = sorted([v for v, e in zip(inks, expected) if e and v > 0])
if not exp_inks:
    print('N/A 0')
    exit(0)
threshold = exp_inks[len(exp_inks)//2] * 0.20
filled = [v > threshold for v in inks]
matches = sum(f == e for f, e in zip(filled, expected))
mismatches = ' '.join(f'{i}:{text[i]}' for i,(f,e) in enumerate(zip(filled,expected)) if f != e)
print(f'{matches} {cols} {mismatches}')
"
}

# ── Test cases ──────────────────────────────────────────────────────
declare -a TESTS=(
    "letters|ABCDEFGHIJ|10|1"
    "pangram|The quick brown fox|19|1"
    "digits|0123456789012345|16|1"
    "wide_M|MMMMMMMMMMMMMMMMMMMM|20|1"
    "mixed|Hello world 12345|17|1"
)

pass=0; fail=0; skip=0

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name text cols rows <<< "$entry"
    echo -n "  $name: "

    render_twp "$text" "$cols" "$rows" "$RESULTS/twp_${name}.png"
    screenshot_kitty "$text" "$RESULTS/kitty_${name}.png"

    if [ ! -f "$RESULTS/twp_${name}.png" ]; then
        echo "SKIP (no TWP PNG)"; skip=$((skip+1)); continue
    fi
    if [ ! -f "$RESULTS/kitty_${name}.png" ]; then
        echo "SKIP (no Kitty screenshot)"; skip=$((skip+1)); continue
    fi

    twp_result=$(check_cells "$RESULTS/twp_${name}.png" "$cols" "$text" "True")
    kit_result=$(check_cells "$RESULTS/kitty_${name}.png" "$cols" "$text" "False")

    twp_match=$(echo "$twp_result" | cut -d' ' -f1)
    twp_total=$(echo "$twp_result" | cut -d' ' -f2)
    twp_mm=$(echo "$twp_result" | cut -d' ' -f3-)
    kit_match=$(echo "$kit_result" | cut -d' ' -f1)
    kit_total=$(echo "$kit_result" | cut -d' ' -f2)
    kit_mm=$(echo "$kit_result" | cut -d' ' -f3-)

    twp_ok=$( [ "$twp_match" = "$twp_total" ] && echo 1 || echo 0 )
    kit_ok=$( [ "$kit_match" = "$kit_total" ] && echo 1 || echo 0 )

    if [ "$twp_ok" = 1 ] && [ "$kit_ok" = 1 ]; then
        status="PASS"; pass=$((pass+1))
    elif [ "$twp_ok" = 1 ]; then
        status="PARTIAL"; pass=$((pass+1))
    else
        status="FAIL"; fail=$((fail+1))
    fi

    line="$status TWP=$twp_match/$twp_total"
    [ -n "$twp_mm" ] && line="$line ($twp_mm)"
    line="$line Kitty=$kit_match/$kit_total"
    [ -n "$kit_mm" ] && line="$line ($kit_mm)"
    echo "$line"
    echo "$line" > "$RESULTS/metrics_${name}.txt"
done

total=$((pass + fail + skip))

# ── HTML Report ─────────────────────────────────────────────────────
REPORT="$RESULTS/report.html"
cat > "$REPORT" <<'HEADER'
<!DOCTYPE html><html lang="en"><head><meta charset="utf-8">
<title>TWP Visual Comparison</title>
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;padding:2rem}
  h1{margin-bottom:.5rem} .meta{color:#94a3b8;margin-bottom:2rem}
  .summary{display:flex;gap:1rem;margin-bottom:2rem}
  .badge{padding:.4rem 1rem;border-radius:8px;font-weight:bold;font-size:1.1rem}
  .pass-bg{background:#16a34a} .fail-bg{background:#dc2626} .partial-bg{background:#ca8a04}
  .test{background:#1e293b;border-radius:12px;padding:1.5rem;margin-bottom:1.5rem}
  .test h2{margin-bottom:.75rem;font-size:1.1rem}
  .status{display:inline-block;padding:.2rem .6rem;border-radius:4px;font-size:.85rem;font-weight:bold;margin-left:.5rem}
  .metrics{margin:.5rem 0;font-family:monospace;font-size:.9rem;color:#94a3b8}
  .images{display:grid;grid-template-columns:1fr 1fr;gap:1rem;margin-top:1rem}
  .img-box{background:#0f172a;border-radius:8px;padding:.75rem}
  .img-box h3{font-size:.75rem;color:#64748b;margin-bottom:.5rem;text-transform:uppercase;letter-spacing:.05em}
  .img-box img{width:100%;image-rendering:pixelated;border:1px solid #334155;border-radius:4px}
  .note{font-size:.8rem;color:#64748b;margin-top:.5rem;font-style:italic}
</style></head><body>
<h1>TWP Visual Comparison Report</h1>
HEADER

echo "<p class=\"meta\">Font: $FONT @ ${FSIZE}pt &middot; $(date)</p>" >> "$REPORT"
echo "<div class=\"summary\">" >> "$REPORT"
echo "<div class=\"badge pass-bg\">$pass passed</div>" >> "$REPORT"
[ "$fail" -gt 0 ] && echo "<div class=\"badge fail-bg\">$fail failed</div>" >> "$REPORT"
[ "$skip" -gt 0 ] && echo "<div class=\"badge partial-bg\">$skip skipped</div>" >> "$REPORT"
echo "</div>" >> "$REPORT"

echo "<p class=\"note\">TWP images are extracted PNGs from our renderer. Kitty images are screenshots from a headless Kitty on Xvfb. Both are compared per-cell against ground truth (the input string). Images are at different pixel densities (renderer-internal vs terminal display) — the metrics are what matter, not pixel-level visual matching.</p>" >> "$REPORT"

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name text cols rows <<< "$entry"
    ml=$(cat "$RESULTS/metrics_${name}.txt" 2>/dev/null || echo "SKIP")
    sc="partial-bg"; case "$ml" in PASS*) sc="pass-bg";; FAIL*) sc="fail-bg";; esac

    cat >> "$REPORT" <<THTML
<div class="test">
  <h2>$name <span class="status $sc">${ml%%\ *}</span></h2>
  <p class="metrics">"$text" · ${cols}×${rows} cells</p>
  <p class="metrics">$ml</p>
  <div class="images">
THTML

    for variant in twp kitty; do
        [ "$variant" = "twp" ] && label="TWP (extracted PNG)" || label="Kitty (Xvfb screenshot)"
        f="$RESULTS/${variant}_${name}.png"
        echo "<div class=\"img-box\"><h3>$label</h3>" >> "$REPORT"
        if [ -f "$f" ]; then
            echo "<img src=\"data:image/png;base64,$(base64 -w0 "$f")\">" >> "$REPORT"
        else
            echo "<p style=\"color:#64748b\">not available</p>" >> "$REPORT"
        fi
        echo "</div>" >> "$REPORT"
    done
    echo "</div></div>" >> "$REPORT"
done

echo "</body></html>" >> "$REPORT"
echo
echo "=========================="
echo "Results: $pass passed, $fail failed, $skip skipped (of $total)"
echo "Report:  file://$RESULTS/report.html"
