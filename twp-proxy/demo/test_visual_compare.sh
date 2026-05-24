#!/usr/bin/env bash
# Visual comparison: Kitty native text vs TWP mono, both in the same
# Kitty terminal running twp-proxy. A single long-lived Kitty instance
# is reused across test runs (PID file + remote-control socket).
#
# i3 rule `for_window [class="^twp-visual-test"] ...` sends the window
# to workspace __twp_test on DP-3.
#
# Dependencies: kitty, xdotool, scrot, python3 + Pillow + numpy
set -e

BINARY="$(pwd)/target/release/twp-proxy"
RESULTS="/tmp/twp-visual-test"
PIDFILE="/tmp/twp-test-kitty.pid"
SOCKPATH="/tmp/twp-test-kitty.sock"
SOCK="unix:$SOCKPATH"
CLASS="twp-visual-test"

FONT=$(grep "^font_family" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_family\s*//')
FSIZE=$(grep "^font_size" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_size\s*//')
: "${FONT:=monospace}"
: "${FSIZE:=16}"
WIN_W=600
WIN_H=200

rm -rf "$RESULTS" && mkdir -p "$RESULTS"

# ── Ensure the long-lived Kitty instance is running ──────────────────
ensure_kitty() {
    # Check if existing instance is alive
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
        echo "  Reusing existing Kitty (PID $(cat "$PIDFILE"))"
        return 0
    fi

    echo "  Launching new Kitty instance on DP-3..."
    rm -f "$SOCKPATH"

    kitty --class="$CLASS" \
          --listen-on="$SOCK" \
          --config=NONE \
          --override="allow_remote_control=yes" \
          --override="font_family=$FONT" \
          --override="font_size=$FSIZE" \
          --override="background=#0a1e24" \
          --override="foreground=#ecefc1" \
          --override="remember_window_size=no" \
          --override="initial_window_width=$WIN_W" \
          --override="initial_window_height=$WIN_H" \
          --override="window_padding_width=0" \
          --override="confirm_os_window_close=0" \
          --override="shell_integration=disabled" \
          "$BINARY" bash 2>/dev/null &
    echo $! > "$PIDFILE"

    # Wait for socket
    for _ in $(seq 1 20); do
        [ -S "$SOCKPATH" ] && break
        sleep 0.2
    done
    sleep 0.5

    echo "  Kitty ready (PID $(cat "$PIDFILE"), socket $SOCKPATH)"
}

# ── Send a command to the Kitty instance and screenshot ──────────────
#   $1 = bash command to run (text to printf)
#   $2 = output PNG path
capture() {
    local cmd="$1" out="$2"
    local orig
    orig=$(xdotool getactivewindow 2>/dev/null || true)

    # Reset terminal + run command in one shot to avoid stale content
    kitty @ --to="$SOCK" send-text "printf '\\\\033c'; sleep 0.2; $cmd\r" 2>/dev/null
    sleep 1.5

    # Brief focus + scrot
    local wid
    wid=$(xdotool search --class "$CLASS" 2>/dev/null | tail -1)
    if [ -n "$wid" ]; then
        xdotool windowfocus --sync "$wid" 2>/dev/null || true
        sleep 0.1
        scrot -u -o "$out" 2>/dev/null || true
        [ -n "$orig" ] && xdotool windowfocus "$orig" 2>/dev/null || true
    fi
}

# ── Cell-fill ground-truth comparison ───────────────────────────────
check_cells() {
    python3 -c "
from PIL import Image
import numpy as np

img = np.array(Image.open('$1').convert('RGB')).astype(float)
text = '''$2'''
cols = $3

expected = [c != ' ' for c in text[:cols]]
while len(expected) < cols: expected.append(False)

bg = img[0, -1, :].copy()

# Find text region
col_dev = np.sqrt(((img - bg)**2).sum(axis=2)).sum(axis=0)
ink_th = col_dev.max() * 0.02
ink_cols = np.where(col_dev > ink_th)[0]
if len(ink_cols) < 2:
    print('0 $3')
    exit(0)
x_off = int(ink_cols[0])
x_end = int(ink_cols[-1]) + 1
cell_w = (x_end - x_off) // cols
if cell_w < 1:
    print('0 $3')
    exit(0)

inks = []
for i in range(cols):
    x0 = x_off + i * cell_w
    x1 = min(x0 + cell_w, img.shape[1])
    if x0 >= img.shape[1]:
        inks.append(0.0); continue
    region = img[:, x0:x1, :]
    inks.append(float(np.sqrt(((region - bg)**2).sum(axis=2)).sum()))

exp_inks = sorted([v for v, e in zip(inks, expected) if e and v > 0])
if not exp_inks:
    print('0 $3')
    exit(0)
threshold = exp_inks[len(exp_inks)//2] * 0.20
filled = [v > threshold for v in inks]
matches = sum(f == e for f, e in zip(filled, expected))
mismatches = ' '.join(f'{i}:{text[i]}' for i,(f,e) in enumerate(zip(filled,expected)) if f != e)
print(f'{matches} $3 {mismatches}')
"
}

# ── Main ────────────────────────────────────────────────────────────
echo "TWP Visual Comparison Test"
echo "=========================="
echo "Font: $FONT @ ${FSIZE}pt"
echo

ensure_kitty

declare -a TESTS=(
    "letters|ABCDEFGHIJ|10"
    "pangram|The quick brown fox|19"
    "digits|0123456789012345|16"
    "wide_M|MMMMMMMMMMMMMMMMMMMM|20"
    "mixed|Hello world 12345|17"
)

pass=0; fail=0; skip=0

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name text cols <<< "$entry"
    echo -n "  $name: "

    # Screenshot 1: native text (just printf, passes through twp-proxy untouched)
    capture "printf '%s' '$text'" "$RESULTS/kitty_${name}.png"

    # Screenshot 2: TWP mono widget
    twp_json="{\"S\":{\"n\":\"mono\",\"t\":\"$text\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}"
    capture "printf '\\\\x1b_twp;v=1,c=$cols,r=1;$twp_json\\\\x1b\\\\\\\\'" "$RESULTS/twp_${name}.png"

    if [ ! -f "$RESULTS/kitty_${name}.png" ] || [ ! -f "$RESULTS/twp_${name}.png" ]; then
        echo "SKIP (screenshot failed)"
        skip=$((skip+1)); continue
    fi

    # Compare both against ground truth
    kit_r=$(check_cells "$RESULTS/kitty_${name}.png" "$text" "$cols")
    twp_r=$(check_cells "$RESULTS/twp_${name}.png" "$text" "$cols")

    kit_match=$(echo "$kit_r" | cut -d' ' -f1)
    twp_match=$(echo "$twp_r" | cut -d' ' -f1)
    kit_mm=$(echo "$kit_r" | cut -d' ' -f3-)
    twp_mm=$(echo "$twp_r" | cut -d' ' -f3-)

    both_ok=$([ "$kit_match" = "$cols" ] && [ "$twp_match" = "$cols" ] && echo 1 || echo 0)
    if [ "$both_ok" = 1 ]; then
        status="PASS"; pass=$((pass+1))
    else
        status="FAIL"; fail=$((fail+1))
    fi

    line="$status Kitty=$kit_match/$cols TWP=$twp_match/$cols"
    [ -n "$kit_mm" ] && line="$line kit:($kit_mm)"
    [ -n "$twp_mm" ] && line="$line twp:($twp_mm)"
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
  .pass-bg{background:#16a34a} .fail-bg{background:#dc2626} .skip-bg{background:#ca8a04}
  .test{background:#1e293b;border-radius:12px;padding:1.5rem;margin-bottom:1.5rem}
  .test h2{margin-bottom:.75rem;font-size:1.1rem}
  .status{display:inline-block;padding:.2rem .6rem;border-radius:4px;font-size:.85rem;font-weight:bold;margin-left:.5rem}
  .metrics{margin:.5rem 0;font-family:monospace;font-size:.9rem;color:#94a3b8}
  .images{display:grid;grid-template-columns:1fr 1fr;gap:1rem;margin-top:1rem}
  .img-box{background:#0f172a;border-radius:8px;padding:.75rem}
  .img-box h3{font-size:.75rem;color:#64748b;margin-bottom:.5rem;text-transform:uppercase;letter-spacing:.05em}
  .img-box img{width:100%;image-rendering:pixelated;border:1px solid #334155;border-radius:4px}
  .note{font-size:.85rem;color:#64748b;margin-bottom:1.5rem;line-height:1.5}
</style></head><body>
<h1>TWP Visual Comparison Report</h1>
HEADER

echo "<p class=\"meta\">Font: $FONT @ ${FSIZE}pt &middot; $(date)</p>" >> "$REPORT"
echo "<p class=\"note\">Both screenshots taken from the same Kitty terminal running <code>twp-proxy</code>. Native text passes through the proxy unchanged; TWP widgets are intercepted and rendered via Kitty Graphics. Same window, same GPU, same pixel density.</p>" >> "$REPORT"

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
  <p class="metrics">"$text" · ${cols} cells</p>
  <p class="metrics">$ml</p>
  <div class="images">
THTML
    for variant in kitty twp; do
        [ "$variant" = "kitty" ] && label="Native text (printf)" || label="TWP mono widget"
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
echo "Kitty:   PID $(cat "$PIDFILE" 2>/dev/null) (kept alive for reuse)"
