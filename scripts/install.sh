#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
native_only=false

case "${1-}" in
    "") ;;
    --native-only) native_only=true ;;
    --help|-h)
        printf '%s\n' \
            'Usage: ./scripts/install.sh [--native-only]' \
            '' \
            'Builds and installs tf into ~/.local/bin.' \
            'By default it also creates a private Python environment for the local' \
            'semantic fallback. --native-only installs only the Rust fast path.'
        exit 0
        ;;
    *)
        printf 'Unknown option: %s\n' "$1" >&2
        exit 2
        ;;
esac

if [ "$(uname -s)" != Darwin ]; then
    printf '%s\n' 'TerFinder currently supports macOS only.' >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' 'Rust/Cargo is required: https://rustup.rs' >&2
    exit 1
fi

cd "$root"
printf '%s\n' '== Building optimized native CLI =='
cargo build --release --locked
"$root/target/release/tf" setup

if [ "$native_only" = true ]; then
    printf '%s\n' 'Installed native parser only.'
    "$HOME/.local/bin/tf" doctor
    exit 0
fi

if [ "$(uname -m)" != arm64 ]; then
    printf '%s\n' 'Local semantic fallback requires Apple Silicon.' >&2
    printf '%s\n' 'Re-run with --native-only on this Mac.' >&2
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    printf '%s\n' 'Python 3.11+ is required for semantic fallback.' >&2
    printf '%s\n' 'The native parser was installed successfully.' >&2
    exit 1
fi

if ! python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)'; then
    printf '%s\n' 'Python 3.11 or newer is required for semantic fallback.' >&2
    printf '%s\n' 'The native parser was installed successfully.' >&2
    exit 1
fi

runtime_dir="$HOME/.local/share/terfinder"
venv="$runtime_dir/venv"
python="$venv/bin/python"

if [ ! -x "$python" ]; then
    printf '%s\n' '== Creating private semantic runtime =='
    mkdir -p "$runtime_dir"
    python3 -m venv "$venv"
fi

if ! "$python" -c 'import laya_mlx' >/dev/null 2>&1; then
    printf '%s\n' '== Installing local semantic classifier =='
    "$python" -m pip install --disable-pip-version-check -r "$root/daemon/requirements.txt"
fi

printf '%s\n' '== Verifying installation =='
"$HOME/.local/bin/tf" doctor
printf '%s\n' '' 'TerFinder is ready. Open a new terminal and try:' \
    '  tf почему мак тормозит' \
    '  tf who is using port 3000'
