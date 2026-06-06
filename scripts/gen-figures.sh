#!/usr/bin/env bash
# Generate the *one* RFC figure that needs the real terminal: the native-ANSI
# vs `term()` palette comparison (§8.2). Its left half is the terminal's own SGR
# rendering, which only the kitty harness (Xvfb + kitty) can capture — the
# in-process renderer can't produce it.
#
# Every OTHER figure and example in RFC.md is rendered in-process by `twp-render`
# and inserted by mdsh — regenerate those with `just docs`, not this script.
set -euo pipefail
cd "$(dirname "$0")/.."

RESULTS="${TWP_RESULTS_DIR:-/tmp/twp-visual-test}"
OUT="docs/figures"
mkdir -p "$OUT"

echo "Running the visual harness (Xvfb + kitty) for the native-palette figure…"
( cd twp-proxy && cargo run --release --bin twp-screenshot -- test ) || true

cp "$RESULTS/kitty_term_palette_gruvbox_dark.png" "$OUT/term-palette-native.png"
cp "$RESULTS/twp_term_palette_gruvbox_dark.png"   "$OUT/term-palette-twp.png"
echo "Wrote $OUT/term-palette-{native,twp}.png"
