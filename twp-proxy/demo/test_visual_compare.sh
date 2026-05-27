#!/usr/bin/env bash
# Visual comparison: Kitty native text vs TWP mono widget.
#
# Both rendered in the same Kitty terminal (running twp-proxy), both
# screenshotted from a headless Xvfb display via ImageMagick `import` —
# same software renderer, same pixel density, same window.
#
# Uses Xvfb + llvmpipe (Mesa software OpenGL) so the test is fully
# self-contained and doesn't touch your real desktop or window manager.
#
# A fresh Kitty instance is launched for each test case to ensure clean
# graphics state under software rendering.
#
# Dependencies: kitty, xvfb (xorg-server-xvfb), xdotool, imagemagick,
#               python3 + Pillow + numpy, mesa (for llvmpipe)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="${SCRIPT_DIR}/../target/release/twp-proxy"
RESULTS="/tmp/twp-visual-test"
CLASS="twp-visual-test"
VDISPLAY=":55"

FONT=$(grep "^font_family" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_family\s*//')
FSIZE=$(grep "^font_size" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_size\s*//')
: "${FONT:=monospace}"
: "${FSIZE:=16}"

export LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe
export KITTY_DISABLE_WAYLAND=1
export DISPLAY="$VDISPLAY"

rm -rf "$RESULTS" && mkdir -p "$RESULTS"

# ── Virtual display ────────────────────────────────────────────────
start_xvfb() {
    echo "  Starting Xvfb on $VDISPLAY..."
    Xvfb "$VDISPLAY" -screen 0 1920x1080x24 +extension GLX +render \
        > /tmp/twp-xvfb.log 2>&1 &
    XVFB_PID=$!

    for _ in $(seq 1 50); do
        xdpyinfo -display "$VDISPLAY" >/dev/null 2>&1 && break
        sleep 0.1
    done

    if ! xdpyinfo -display "$VDISPLAY" >/dev/null 2>&1; then
        echo "ERROR: Xvfb failed to start" >&2
        cat /tmp/twp-xvfb.log >&2
        exit 1
    fi
    echo "  Xvfb ready (PID $XVFB_PID)"
}

cleanup() {
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

# ── Run one render in a fresh Kitty instance ───────────────────────
# Usage: run_render <script_path> <output_png>
run_render() {
    local script="$1" output="$2"
    local sig_file="$RESULTS/_render_sig_$$"
    local kitty_pid

    rm -f "$sig_file"

    kitty --class="$CLASS" \
          --config=NONE \
          --override="allow_remote_control=yes" \
          --override="font_family=$FONT" \
          --override="font_size=$FSIZE" \
          --override="background=#0a1e24" \
          --override="foreground=#ecefc1" \
          --override="remember_window_size=no" \
          --override="initial_window_width=60c" \
          --override="initial_window_height=10c" \
          --override="confirm_os_window_close=0" \
          --override="shell_integration=disabled" \
          --override="window_padding_width=0" \
          "$BINARY" bash -c "
              printf '\x1b[?25l\x1b[2J\x1b[H'
              sleep 0.3
              $script
              touch $sig_file
              sleep 120
          " 2>/dev/null &
    kitty_pid=$!

    for _ in $(seq 1 100); do [ -f "$sig_file" ] && break; sleep 0.2; done

    local wid
    for _ in $(seq 1 10); do
        sleep 1
        wid=$(xdotool search --class "$CLASS" 2>/dev/null | tail -1)
        if [ -n "$wid" ]; then
            import -window "$wid" "$output" 2>/dev/null
            local size
            size=$(stat -c%s "$output" 2>/dev/null || echo 0)
            [ "$size" -gt 500 ] && break
        fi
    done

    kill "$kitty_pid" 2>/dev/null || true
    wait "$kitty_pid" 2>/dev/null || true
    rm -f "$sig_file"
}


# ── Test cases ──────────────────────────────────────────────────────
declare -a TESTS=(
    "letters|ABCDEFGHIJ|10"
    "pangram|The quick brown fox|19"
    "digits|0123456789012345|16"
    "wide_M|MMMMMMMMMMMMMMMMMMMM|20"
    "mixed|Hello world 12345|17"
)

# ── Main ────────────────────────────────────────────────────────────
echo "TWP Visual Comparison Test"
echo "=========================="
echo "Font: $FONT @ ${FSIZE}pt"
echo

start_xvfb

pass=0; fail=0; skip=0

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name text cols <<< "$entry"
    echo -n "  $name: "

    # Native render
    run_render "printf '%s' '$text'" "$RESULTS/kitty_${name}.png"

    # TWP render
    run_render "printf '\x1b_twp;v=1,c=$cols,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"$text\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" "$RESULTS/twp_${name}.png"

    if [ ! -f "$RESULTS/kitty_${name}.png" ] || [ ! -f "$RESULTS/twp_${name}.png" ]; then
        echo "SKIP (screenshot failed)"
        skip=$((skip+1)); continue
    fi

    # Compare: Kitty is reference, TWP must match its cell-fill pattern
    result=$(python3 -c "
from PIL import Image
import numpy as np

kit = np.array(Image.open('$RESULTS/kitty_${name}.png').convert('RGB')).astype(float)
twp = np.array(Image.open('$RESULTS/twp_${name}.png').convert('RGB')).astype(float)
cols = $cols
text = '''$text'''

def cell_fill(img, ncols):
    bg = img[0, -1, :].copy()
    col_dev = np.sqrt(((img - bg)**2).sum(axis=2)).sum(axis=0)
    ink_th = col_dev.max() * 0.02
    ink_cols = np.where(col_dev > ink_th)[0]
    if len(ink_cols) < 2:
        return [False] * ncols, 0
    x0 = int(ink_cols[0])
    x1 = int(ink_cols[-1]) + 1
    cw = (x1 - x0) // ncols
    if cw < 1:
        return [False] * ncols, 0
    inks = []
    for i in range(ncols):
        cx0 = x0 + i * cw
        cx1 = min(cx0 + cw, img.shape[1])
        if cx0 >= img.shape[1]:
            inks.append(0.0); continue
        inks.append(float(np.sqrt(((img[:, cx0:cx1, :] - bg)**2).sum(axis=2)).sum()))
    nonzero = sorted([v for v in inks if v > 0])
    if not nonzero:
        return [False] * ncols, 0
    threshold = nonzero[len(nonzero)//2] * 0.20
    return [v > threshold for v in inks], sum(v > threshold for v in inks)

kit_cells, kit_n = cell_fill(kit, cols)
twp_cells, twp_n = cell_fill(twp, cols)

expected_filled = sum(1 for c in text[:cols] if c != ' ')
min_ink = max(expected_filled // 2, 1)

if kit_n < min_ink:
    print(f'FAIL kitty blank ({kit_n}/{expected_filled})')
    exit(0)
if twp_n < min_ink:
    print(f'FAIL twp blank ({twp_n}/{expected_filled})')
    exit(0)

matches = sum(k == t for k, t in zip(kit_cells, twp_cells))
mm = [f'{i}:{text[i] if i<len(text) else \"?\"}' for i,(k,t) in enumerate(zip(kit_cells, twp_cells)) if k != t]
status = 'PASS' if matches == cols else 'FAIL'
detail = f'{matches}/{cols}'
if mm: detail += ' mm=' + ','.join(mm[:5])
print(f'{status} {detail}')
")
    echo "$result"
    echo "$result" > "$RESULTS/metrics_${name}.txt"

    case "$result" in PASS*) pass=$((pass+1)) ;; *) fail=$((fail+1)) ;; esac
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
  .pass-bg{background:#16a34a} .fail-bg{background:#dc2626} .skip-bg{background:#ca8a04}
  .test{background:#1e293b;border-radius:12px;padding:1.5rem;margin-bottom:1.5rem}
  .test h2{margin-bottom:.75rem;font-size:1.1rem}
  .status{display:inline-block;padding:.2rem .6rem;border-radius:4px;font-size:.85rem;font-weight:bold;margin-left:.5rem}
  .metrics{margin:.5rem 0;font-family:monospace;font-size:.9rem;color:#94a3b8}
  .images{display:grid;grid-template-columns:1fr 1fr;gap:1rem;margin-top:1rem}
  .img-box{background:#0f172a;border-radius:8px;padding:.75rem}
  .img-box h3{font-size:.75rem;color:#64748b;margin-bottom:.5rem;text-transform:uppercase;letter-spacing:.05em}
  .img-box img{width:100%;image-rendering:auto;border:1px solid #334155;border-radius:4px}
  .note{font-size:.85rem;color:#64748b;margin-bottom:1.5rem;line-height:1.5}
</style></head><body>
<h1>TWP Visual Comparison Report</h1>
HEADER

echo "<p class=\"meta\">Font: $FONT @ ${FSIZE}pt &middot; $(date)</p>" >> "$REPORT"
echo "<p class=\"note\">Both screenshots from the same Kitty terminal running <code>twp-proxy</code> on a headless Xvfb display (llvmpipe software rendering). Captured via ImageMagick <code>import</code>. Native text passes through the proxy unchanged; TWP widgets are intercepted and rendered via Kitty Graphics.</p>" >> "$REPORT"

echo "<div class=\"summary\">" >> "$REPORT"
echo "<div class=\"badge pass-bg\">$pass passed</div>" >> "$REPORT"
[ "$fail" -gt 0 ] && echo "<div class=\"badge fail-bg\">$fail failed</div>" >> "$REPORT"
[ "$skip" -gt 0 ] && echo "<div class=\"badge skip-bg\">$skip skipped</div>" >> "$REPORT"
echo "</div>" >> "$REPORT"

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name text cols <<< "$entry"
    ml=$(cat "$RESULTS/metrics_${name}.txt" 2>/dev/null || echo "SKIP")
    sc="skip-bg"; case "$ml" in PASS*) sc="pass-bg";; FAIL*) sc="fail-bg";; esac

    cat >> "$REPORT" <<THTML
<div class="test">
  <h2>$name <span class="status $sc">${ml%%\ *}</span></h2>
  <p class="metrics">"$text" &middot; ${cols} cells &middot; $ml</p>
  <div class="images">
THTML
    for variant in kitty twp; do
        [ "$variant" = "kitty" ] && label="Kitty native" || label="TWP mono"
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
