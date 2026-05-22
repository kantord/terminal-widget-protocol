#!/usr/bin/env bash
# Demonstration of the Terminal Widget Protocol hello-world.
# Run this script inside `twp-proxy zsh` (or bash) under Kitty to see widgets
# render inline with surrounding text.

set -e

echo "TWP hello-world demo"
echo "===================="
echo
echo "Emitting a triangle widget:"
printf '\x1b_twp;foo\x1b\\'
echo
echo
echo "Emitting a circle widget:"
printf '\x1b_twp;bar\x1b\\'
echo
echo
echo "Both widgets inline in text:"
echo "Here is a triangle: $(printf '\x1b_twp;foo\x1b\\') and a circle: $(printf '\x1b_twp;bar\x1b\\')"
echo
echo "Done."
