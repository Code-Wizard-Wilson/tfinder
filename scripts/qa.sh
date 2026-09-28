#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

echo "== Shell scripts =="
sh -n scripts/install.sh scripts/tf-shell.sh

echo "== Rust formatting =="
cargo fmt --all --check

echo "== Rust lint =="
cargo clippy --all-targets -- -D warnings

echo "== Rust tests =="
cargo test --all-targets

echo "== Release-mode Rust tests =="
cargo test --release --all-targets

echo "== Python daemon tests =="
python3 -m unittest discover -s daemon -p 'test_*.py'

echo "== Release build =="
cargo build --release

bin=./target/release/tf

echo "== Fixture evaluation =="
python3 daemon/eval.py "$bin" --fast-misses-only --require-perfect

if [ -x daemon/.venv/bin/python ]; then
    echo "== Conversational semantic evaluation =="
    TERFINDER_PYTHON="$PWD/daemon/.venv/bin/python" \
        daemon/.venv/bin/python daemon/eval.py "$bin" --conversational --summary --require-perfect
else
    echo "== Conversational semantic evaluation skipped: daemon/.venv is absent =="
fi

expect_action() {
    expected=$1
    shift
    actual=$($bin --fast-only --interpret-only "$@" | python3 -c 'import json,sys; print(json.load(sys.stdin)["action"])')
    if [ "$actual" != "$expected" ]; then
        echo "expected $expected, got $actual for: $*" >&2
        exit 1
    fi
}

echo "== CLI smoke matrix =="
expect_action SHOW_SYSTEM_INFO system info
expect_action SHOW_MEMORY memory usage
expect_action SHOW_CPU какой процессор
expect_action SHOW_NETWORK покажи ip адрес
expect_action LIST_PROCESSES какой процесс занимает больше всего памяти
expect_action LIST_PROCESSES top CPU processes
expect_action SET_BLUETOOTH_POWER выключи блютус
expect_action SET_BLUETOOTH_POWER disable bluetoth
expect_action SET_AIRDROP_MODE включи AirDrop
expect_action SET_AIRDROP_MODE AirDrop только для контактов
expect_action SET_STAGE_MANAGER включи StageManager
expect_action SET_STAGE_MANAGER turn off Stage Manager
expect_action SET_STAGE_MANAGER turn the StageManager on
expect_action FETCH_URL сделай curl запрос на 'https://example.com?a=1&b=2'
expect_action FETCH_URL отправь запрос на google.com
expect_action FETCH_URL отправь запрос curl на google.com
expect_action COPY_FILE копируй a.txt в b.txt
expect_action COPY_FILE copia a.txt a b.txt
expect_action COPY_FILE kopiere a.txt nach b.txt
expect_action COPY_FILE انسخ a.txt إلى b.txt
expect_action COPY_FILE कॉपी करो a.txt में b.txt
expect_action RENAME_FILE renomme a.txt en b.txt
expect_action MOVE_FILE verplaats a.txt naar b.txt
expect_action DELETE_DIRECTORY удали папку build
expect_action SHOW_BATTERY كم تبقى من البطارية؟

if ! $bin model status >/dev/null; then
    echo "model status must be a safe, non-erroring inspection command" >&2
    exit 1
fi

negated=$($bin --fast-only --interpret-only 'Please do not open Safari' | python3 -c 'import json,sys; print(json.load(sys.stdin)["action"])')
if [ "$negated" != "UNSUPPORTED" ]; then
    echo "explicitly negated action was unexpectedly accepted" >&2
    exit 1
fi

if $bin --fast-only --interpret-only 'I wonder whether port 5173 is a nice number' >/dev/null 2>&1; then
    echo "incidental port number was unexpectedly accepted" >&2
    exit 1
fi

capability_count=$($bin capabilities --json | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
if [ "$capability_count" -lt 39 ]; then
    echo "capability registry unexpectedly shrank to $capability_count" >&2
    exit 1
fi

shell_action=$($bin --fast-only --interpret-only 'git status; touch /tmp/terfinder-must-not-exist' | python3 -c 'import json,sys; print(json.load(sys.stdin)["action"])')
if [ "$shell_action" != "UNSUPPORTED" ]; then
    echo "shell-like tool input was unexpectedly mapped to $shell_action" >&2
    exit 1
fi

echo "== High-volume black-box stress matrix =="
python3 scripts/stress_qa.py "$bin"

echo "QA passed: $capability_count capabilities"
