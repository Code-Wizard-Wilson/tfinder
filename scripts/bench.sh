#!/bin/sh
set -eu

binary="${1:-target/release/tf}"
if [ ! -x "$binary" ]; then
  echo "Build the release binary first: cargo build --release" >&2
  exit 1
fi

echo "Binary bytes: $(wc -c < "$binary" | tr -d ' ')"
echo "Version latency (100 iterations):"
/usr/bin/time -p sh -c 'i=0; while [ "$i" -lt 100 ]; do "$1" --version >/dev/null; i=$((i + 1)); done' sh "$binary"
echo "Parser-only port lookup latency (20 iterations):"
/usr/bin/time -p sh -c 'i=0; while [ "$i" -lt 20 ]; do "$1" port 8765 >/dev/null; i=$((i + 1)); done' sh "$binary"
echo "Parser-only maximum RSS from /usr/bin/time -l:"
/usr/bin/time -l "$binary" --version >/dev/null
echo "Model and daemon metrics require a Metal-enabled interactive session."
