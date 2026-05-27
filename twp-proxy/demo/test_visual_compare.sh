#!/usr/bin/env bash
# Visual comparison: Kitty native text vs TWP mono widget.
#
# Tests both basic text rendering and Kitty's text-sizing protocol (OSC 66)
# parameters: scale, char-width, and subscale.
#
# Uses twp-screenshot (Rust) for rendering and Xvfb + llvmpipe for headless
# display. Comparison and reporting remain in bash/python.
#
# Dependencies: kitty (≥0.35 for OSC 66), xvfb (xorg-server-xvfb),
#               xdotool, imagemagick, python3 + Pillow + numpy,
#               mesa (for llvmpipe)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROXY="${SCRIPT_DIR}/../target/release/twp-proxy"
SCREENSHOT="${SCRIPT_DIR}/../target/release/twp-screenshot"
RESULTS="/tmp/twp-visual-test"
VDISPLAY=":55"

FONT=$(grep "^font_family" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_family\s*//')
FSIZE=$(grep "^font_size" ~/.config/kitty/kitty.conf 2>/dev/null | head -1 | sed 's/^font_size\s*//')
: "${FONT:=monospace}"
: "${FSIZE:=16}"

rm -rf "$RESULTS" && mkdir -p "$RESULTS"

# ── Virtual display ────────────────────────────────────────────────
start_xvfb() {
    echo "  Starting Xvfb on $VDISPLAY..."
    Xvfb "$VDISPLAY" -screen 0 1920x1080x24 +extension GLX +render \
        > /tmp/twp-xvfb.log 2>&1 &
    XVFB_PID=$!

    for _ in $(seq 1 50); do
        DISPLAY="$VDISPLAY" xdpyinfo >/dev/null 2>&1 && break
        sleep 0.1
    done

    if ! DISPLAY="$VDISPLAY" xdpyinfo >/dev/null 2>&1; then
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

# ── Screenshot helpers using twp-screenshot ────────────────────────
ss_common=(--display "$VDISPLAY" --font "$FONT" --font-size "$FSIZE")

ss_native() {
    "$SCREENSHOT" "${ss_common[@]}" --output "$1" -- "$2"
}

ss_native_bare() {
    "$SCREENSHOT" "${ss_common[@]}" --output "$1" -- "$2"
}

ss_twp() {
    "$SCREENSHOT" "${ss_common[@]}" --proxy "$PROXY" --output "$1" -- "$2"
}

# ── Compare helper ─────────────────────────────────────────────────
compare() {
    local name="$1" text="$2" cols="$3"
    python3 -c "
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
"
}

# ── Test runner ────────────────────────────────────────────────────
# run_test <name> <text> <cols> <native_cmd> <twp_cmd> [native_use_proxy]
run_test() {
    local name="$1" text="$2" cols="$3" native_cmd="$4" twp_cmd="$5"
    local native_proxy="${6:-yes}"

    echo -n "  $name: "

    if [ "$native_proxy" = "yes" ]; then
        ss_twp "$RESULTS/kitty_${name}.png" "$native_cmd" || true
    else
        ss_native "$RESULTS/kitty_${name}.png" "$native_cmd" || true
    fi
    ss_twp "$RESULTS/twp_${name}.png" "$twp_cmd" || true

    if [ ! -f "$RESULTS/kitty_${name}.png" ] || [ ! -f "$RESULTS/twp_${name}.png" ]; then
        echo "SKIP (screenshot failed)"
        echo "SKIP" > "$RESULTS/metrics_${name}.txt"
        return 0
    fi

    local result
    result=$(compare "$name" "$text" "$cols")
    echo "$result"
    echo "$result" > "$RESULTS/metrics_${name}.txt"
    return 0
}


# ── Main ────────────────────────────────────────────────────────────
echo "TWP Visual Comparison Test"
echo "=========================="
echo "Font: $FONT @ ${FSIZE}pt"
echo

start_xvfb

ALL_TESTS=()
record() { ALL_TESTS+=("$1"); }

# ── Basic mono tests (scale=1, native = plain printf) ──────────────
echo "── Basic mono (scale=1) ──"

run_test "letters" "ABCDEFGHIJ" 10 \
    "printf '%s' 'ABCDEFGHIJ'" \
    "printf '\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDEFGHIJ\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'"
record letters

run_test "pangram" "The quick brown fox" 19 \
    "printf '%s' 'The quick brown fox'" \
    "printf '\x1b_twp;v=1,c=19,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"The quick brown fox\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'"
record pangram

run_test "digits" "0123456789012345" 16 \
    "printf '%s' '0123456789012345'" \
    "printf '\x1b_twp;v=1,c=16,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"0123456789012345\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'"
record digits

run_test "wide_M" "MMMMMMMMMMMMMMMMMMMM" 20 \
    "printf '%s' 'MMMMMMMMMMMMMMMMMMMM'" \
    "printf '\x1b_twp;v=1,c=20,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"MMMMMMMMMMMMMMMMMMMM\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'"
record wide_M

run_test "mixed" "Hello world 12345" 17 \
    "printf '%s' 'Hello world 12345'" \
    "printf '\x1b_twp;v=1,c=17,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"Hello world 12345\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'"
record mixed

echo

# ── Text-sizing tests (OSC 66 vs TWP mono sizing params) ───────────
echo "── Text-sizing (OSC 66 vs TWP) ──"

run_test "scale2" "ABCDE" 10 \
    "printf '\x1b]66;s=2;ABCDE\x07'" \
    "printf '\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record scale2

run_test "scale3" "ABC" 9 \
    "printf '\x1b]66;s=3;ABC\x07'" \
    "printf '\x1b_twp;v=1,c=9,r=3;{\"S\":{\"n\":\"mono\",\"t\":\"ABC\",\"s\":{\"scale\":3,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record scale3

run_test "charw2" "ABCDE" 10 \
    "printf '\x1b]66;w=2;ABCDE\x07'" \
    "printf '\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"char-width\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record charw2

run_test "sub_half" "ABCDEFGHIJ" 10 \
    "printf '\x1b]66;n=1:d=2;ABCDEFGHIJ\x07'" \
    "printf '\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDEFGHIJ\",\"s\":{\"subscale-n\":1,\"subscale-d\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record sub_half

run_test "scale2_sub_half" "ABCDE" 10 \
    "printf '\x1b]66;s=2:n=1:d=2;ABCDE\x07'" \
    "printf '\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"scale\":2,\"subscale-n\":1,\"subscale-d\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record scale2_sub_half

run_test "scale2_digits" "0123456789" 20 \
    "printf '\x1b]66;s=2;0123456789\x07'" \
    "printf '\x1b_twp;v=1,c=20,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"0123456789\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\x1b\\\\'" \
    "no"
record scale2_digits

echo

# Count results
pass=0; fail=0; skip=0
for name in "${ALL_TESTS[@]}"; do
    ml=$(cat "$RESULTS/metrics_${name}.txt" 2>/dev/null || echo "SKIP")
    case "$ml" in PASS*) pass=$((pass+1)) ;; SKIP*) skip=$((skip+1)) ;; *) fail=$((fail+1)) ;; esac
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
  h2.section{margin:1.5rem 0 1rem;font-size:1.3rem;border-bottom:1px solid #334155;padding-bottom:.5rem}
</style></head><body>
<h1>TWP Visual Comparison Report</h1>
HEADER

echo "<p class=\"meta\">Font: $FONT @ ${FSIZE}pt &middot; $(date)</p>" >> "$REPORT"
echo "<p class=\"note\">Screenshots from Kitty running <code>twp-proxy</code> on a headless Xvfb display (llvmpipe software rendering). Captured via <code>twp-screenshot</code>. Basic tests compare native text vs TWP mono widget. Text-sizing tests compare Kitty OSC 66 output vs TWP mono with equivalent <code>scale</code>, <code>char-width</code>, and <code>subscale</code> parameters.</p>" >> "$REPORT"

echo "<div class=\"summary\">" >> "$REPORT"
echo "<div class=\"badge pass-bg\">$pass passed</div>" >> "$REPORT"
[ "$fail" -gt 0 ] && echo "<div class=\"badge fail-bg\">$fail failed</div>" >> "$REPORT"
[ "$skip" -gt 0 ] && echo "<div class=\"badge skip-bg\">$skip skipped</div>" >> "$REPORT"
echo "</div>" >> "$REPORT"

section_printed=0
for name in "${ALL_TESTS[@]}"; do
    if [ "$section_printed" -eq 0 ] && [[ "$name" != scale* ]] && [[ "$name" != charw* ]] && [[ "$name" != sub_* ]]; then
        echo "<h2 class=\"section\">Basic mono (scale=1)</h2>" >> "$REPORT"
    fi
    if [ "$section_printed" -eq 0 ] && { [[ "$name" == scale* ]] || [[ "$name" == charw* ]] || [[ "$name" == sub_* ]]; }; then
        echo "<h2 class=\"section\">Text-sizing (OSC 66 vs TWP)</h2>" >> "$REPORT"
        section_printed=1
    fi

    ml=$(cat "$RESULTS/metrics_${name}.txt" 2>/dev/null || echo "SKIP")
    sc="skip-bg"; case "$ml" in PASS*) sc="pass-bg";; FAIL*) sc="fail-bg";; esac

    cat >> "$REPORT" <<THTML
<div class="test">
  <h2>$name <span class="status $sc">${ml%%\ *}</span></h2>
  <p class="metrics">$ml</p>
  <div class="images">
THTML
    for variant in kitty twp; do
        if [[ "$name" == scale* ]] || [[ "$name" == charw* ]] || [[ "$name" == sub_* ]]; then
            [ "$variant" = "kitty" ] && label="Kitty OSC 66" || label="TWP mono"
        else
            [ "$variant" = "kitty" ] && label="Kitty native" || label="TWP mono"
        fi
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
echo "=========================="
echo "Results: $pass passed, $fail failed, $skip skipped (of $total)"
echo "Report:  file://$RESULTS/report.html"
