#!/usr/bin/env bash
# Regenerate the figures embedded in RFC.md from the reference polyfill's own
# output. This runs the visual test harness (which needs Xvfb + kitty) and
# copies a curated subset of its real captures into docs/figures/.
#
# The figures in the spec are therefore not mock-ups — they are exactly what
# `twp-proxy` renders. Run from anywhere:  scripts/gen-figures.sh
set -euo pipefail
cd "$(dirname "$0")/.."

RESULTS="${TWP_RESULTS_DIR:-/tmp/twp-visual-test}"
OUT="docs/figures"
mkdir -p "$OUT"

echo "Running the visual harness (Xvfb + kitty)…"
( cd twp-proxy && cargo run --release --bin twp-screenshot -- test ) || true

copy() { cp "$RESULTS/$1" "$OUT/$2" && echo "  $OUT/$2"; }
echo "Copying curated figures:"
copy twp_docker_dashboard_gruvbox_dark.png    docker-dashboard-dark.png
copy twp_docker_dashboard_solarized_light.png docker-dashboard-light.png
copy twp_diff_review_dracula.png              diff-review.png
copy twp_now_playing_bar.png                  now-playing.png
copy twp_app_line_chart.png                   svg-line-chart.png
copy kitty_term_palette_gruvbox_dark.png      term-palette-native.png
copy twp_term_palette_gruvbox_dark.png        term-palette-twp.png

echo "Done. Figures are referenced from RFC.md."
