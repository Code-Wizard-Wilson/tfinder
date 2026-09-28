# TerFinder

**Natural-language control for macOS — fast, local, and safe by design.**

Tell your terminal what you want in plain English (or 16 other languages). TerFinder maps the request to a registered, typed action instead of generating arbitrary shell commands.

[Quick start](#quick-start) · [Examples](#examples) · [Safety](#safety) · [How it works](#how-it-works) · [Development](#development) · [Changelog](CHANGELOG.md)

```text
$ tf why is my Mac slow
Top processes by CPU
   PID     CPU      MEM       RSS  COMMAND
   377    16.8%     0.7%     121.1 MiB  WindowServer

$ tf what owns PID 46272
PID 46272 · /Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge

$ tf turn off Bluetooth
Turn Bluetooth off
Continue? [y/N]
```

## Why TerFinder

- **Natural language** — ask for the result instead of remembering commands and flags.
- **Fast** — common requests are parsed directly by the native Rust path.
- **Local** — semantic fallback runs locally on supported Apple Silicon Macs.
- **Safe** — state-changing actions are previewed and require confirmation.
- **Typed** — the model chooses only from registered actions; it cannot emit shell code.
- **Multilingual** — 17 supported languages, mixed-language requests, colloquialisms, and common typos.
## Quick start

### Requirements

- macOS
- Rust 1.85+
- Python 3.11+ for semantic fallback
- Apple Silicon + Metal for the local Laya-MLX model
- `blueutil` for Bluetooth power control: `brew install blueutil`

### Install

```sh
git clone https://github.com/Code-Wizard-Wilson/tfinder.git
cd tfinder
./scripts/install.sh
```

The installer builds the optimized Rust binary, installs `tf` and `terfinder` to `~/.local/bin`, configures shell integration for natural-language `cd`, and creates a private semantic runtime.

If you only want the native Rust path:

```sh
./scripts/install.sh --native-only
```

Then open a new terminal and verify the installation:

```sh
tf doctor
tf model status
```

> The first semantic request may download roughly 650 MB of model weights. Common requests do not load the model.

## Examples

```sh
tf show me what's in Downloads
tf show me what's in README.md
tf copy a.txt to Documents
tf open Safari
tf what has grabbed TCP port 8765
tf which process is using the most memory
tf how much battery do I have left?
tf turn on AirDrop
tf turn off Stage Manager
tf fetch example.com
```
TerFinder also understands the same actions across Russian, Ukrainian, Spanish, German, French, Portuguese, Italian, Polish, Turkish, Dutch, Czech, Chinese, Japanese, Korean, Arabic, and Hindi.

Run `tf` with no arguments for an interactive session:

```text
$ tf
> what is using port 8765?
...
> kill it
Terminate process ...
Continue? [y/N]
```

Session context is kept only in memory and is discarded when you exit.

## What it can do

TerFinder currently provides 40 registered capabilities, including:

- Open, locate, and quit macOS apps
- Find, read, copy, move, rename, and Trash files
- List and create directories
- Resolve natural-language directory changes
- Open URLs and perform bounded HTTP/HTTPS GET requests
- Inspect processes and TCP listeners
- Terminate a PID or a port's single listener with SIGTERM
- Inspect disk, battery, memory, CPU, uptime, hostname, user, macOS version, and local addresses
- Find large files and calculate directory sizes
- Clear supported pip/npm/yarn/pnpm/Homebrew caches
- Control Bluetooth, AirDrop receiving, and Stage Manager

A bare target also works where it is unambiguous:

```sh
tf telegram
tf .
```

## Safety

TerFinder is intentionally **not** a general-purpose shell agent.

- No `sh -c`, `eval`, or arbitrary command executor
- Model output never becomes shell text
- Mutations show the exact plan before execution
- Destructive actions require confirmation
- `--dry-run` resolves and displays a plan without executing it
- Ambiguous or low-confidence requests execute nothing
- Explicitly negated actions are refused
- PID 0, PID 1, the CLI itself, and its parent process are protected
- Critical roots such as `/`, `/System`, `/Applications`, `/Library`, `/Users`, and your home directory cannot be moved to Trash
- File/app deletion uses `~/.Trash`; permanent deletion is not implemented

`--yes` can skip confirmation for ordinary destructive actions, but it does **not** enable permanent deletion.
File moves use macOS `renamex_np` with `RENAME_EXCL`, preventing a destination created after preview from being overwritten. Process targets are rechecked immediately before SIGTERM.

HTTP requests are deliberately bounded: only HTTP(S), GET only, connection/total timeouts, a 5 MiB response cap, and no request bodies or credentials in URLs.

## How it works

```text
Natural-language request
        │
        ▼
┌─────────────────────┐
│ Native Rust parser  │  ← common requests
└──────────┬──────────┘
           │ abstains
           ▼
┌─────────────────────┐
│ Local Laya-MLX model│  ← semantic fallback
└──────────┬──────────┘
           │ chooses one registered capability
           ▼
┌─────────────────────┐
│ Typed action + slots│
└──────────┬──────────┘
           ▼
   validation / preview
           │
           ▼
      safe executor
```

The semantic model is `aac6fef/laya-multilingual-mlx`. It starts only when the native parser abstains and exits after 600 idle seconds by default. There is no permanent background service, network listener, or telemetry.

The default decision threshold is `0.70`. It is an abstention threshold, not a calibrated probability. If TerFinder cannot safely extract required parameters, it reports what it understood and does nothing.

### Configuration

Optional config: `~/.config/terfinder/config.toml`

```toml
[intent]
confidence_threshold = 0.70
fast_parser = true

[model]
name = "multilingual"
idle_timeout_seconds = 600
transport = "unix"
autostart = true
# python = "/absolute/path/to/venv/bin/python"
```

The model may also be `english` or `typed-decisions`. `TERFINDER_PYTHON` overrides the configured Python path.
## HTTP proxies

HTTP GET requests honor standard proxy environment variables:

```sh
env HTTPS_PROXY=http://127.0.0.1:8080 tf fetch example.com
env ALL_PROXY=socks5h://127.0.0.1:1080 tf fetch example.com
```

Proxy settings are not accepted as arbitrary curl flags.

## Phrase knowledge

TerFinder includes a multilingual term index and anchored phrase rules. Maintainers can extend local phrasing without adding shell execution.

```json
{
  "version": 1,
  "prefixes": ["please", "could you", "kindly"],
  "terms": {
    "open": ["launch", "activate"],
    "find": ["locate", "track down"]
  },
  "rules": [
    { "match": "wipe the {target} cache", "rewrite": "clear cache {target}" }
  ]
}
```

Run `tf knowledge` to inspect and validate the knowledge base. Rewritten phrases still have to resolve to a registered typed action and pass normal safety checks.

## Testing

The release gate covers **7,192 distinct user scenarios** across all 40 capabilities, plus a separate **5,000-run endurance loop**.

Coverage includes multilingual and conversational requests, typos, negation, ambiguity, adversarial input, apps, files, directories, processes, ports, caches, URLs, system controls, HTTP limits, dry runs, and state checks.

See [Evaluation](docs/EVALUATION.md) for details.

## Development

```sh
cargo test
python3 -m unittest discover -s daemon -p 'test_*.py'
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
scripts/qa.sh
```

Additional evaluation and benchmark commands:

```sh
scripts/bench.sh
python3 scripts/stress_qa.py ./target/release/tf
python3 daemon/eval.py ./target/release/tf --fast-misses-only
python3 daemon/eval.py ./target/release/tf --conversational --summary
```
## Current limitations

- General multi-step planning is not implemented; only specific preflighted multi-step flows are supported.
- Interactive context currently covers a limited set of references such as previously shown ports and apps.
- Natural language is unbounded, so uncertain requests intentionally abstain.
- The multilingual semantic model uses roughly 647 MB on disk and about 644 MB while loaded on the validation Mac.
- Trash moves across different filesystems may fail cleanly rather than falling back to permanent deletion.
- Recursive search is bounded to the requested project or directory; there is no implicit full-volume scan.
- Privileged app removal, permanent deletion, default-browser changes, arbitrary downloads, and arbitrary shell commands are not supported.

## License

[MIT](LICENSE)
