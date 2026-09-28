# TerFinder

**Say what you need. TerFinder turns it into a safe, typed macOS action.**

[Install](#install-and-run) · [Actions and safety](#actions-and-safety) · [Evaluation](docs/EVALUATION.md) · [Changelog](CHANGELOG.md)

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

Use Russian, English, Ukrainian, Spanish, German, French, Portuguese, Italian, Polish, Turkish, Dutch, Czech, Chinese, Japanese, Korean, Arabic, Hindi, or mix languages in one request. Common requests stay on a fast native Rust path. Unfamiliar wording falls through to a local multilingual classifier that can only choose from 40 registered actions. It never generates a shell command.

Mutations show an exact plan and require confirmation. Explicit negation is refused. Ambiguous or low-confidence requests execute nothing. The permanent release gate covers 7,192 distinct user scenarios plus a 5,000-run endurance loop.

## Why TerFinder

- **No command vocabulary:** ask for an outcome instead of remembering flags.
- **Local by default:** common requests are native; semantic fallback runs locally on Apple Silicon.
- **Typed execution:** model output cannot become arbitrary shell text.
- **Safe changes:** previews, target validation, confirmations, dry runs, and protected system paths.
- **Actually multilingual:** the same actions work across 17 languages, mixed phrasing, colloquialisms, and common typos.

## Install and run

Requires macOS, Rust 1.85 or newer, and Python 3.11 or newer for semantic fallback. Laya-MLX requires Apple Silicon with working Metal access. Bluetooth power control requires `blueutil` (`brew install blueutil`). The first unfamiliar request may download `aac6fef/laya-multilingual-mlx` to the Hugging Face cache.

```sh
./scripts/install.sh
```

The installer builds the optimized Rust binary, copies `tf` and `terfinder` to `~/.local/bin`, adds the shell integration required for natural-language directory changes, and creates a private semantic runtime in `~/.local/share/terfinder/venv`. Use `./scripts/install.sh --native-only` to install only the fast Rust path. Add `~/.local/bin` to `PATH` if needed, open a new terminal, then verify with:

```sh
tf doctor
tf model status
```

The CLI installs no login item, watcher, network listener, or permanent background service. Semantic fallback starts only when the native parser abstains and exits after 600 idle seconds. The first semantic request may download roughly 650 MB of model weights; common requests do not load the model.

Manual native-only installation remains available:

```sh
cargo build --release --locked
./target/release/tf setup
```

You can set `[model] python = "/absolute/path/to/venv/bin/python"` in `~/.config/terfinder/config.toml` instead of using the managed runtime or exporting `TERFINDER_PYTHON`. The environment variable takes precedence.

Optional `~/.config/terfinder/config.toml`:

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

The model may also be `english` or `typed-decisions`. TerFinder refuses a running daemon with a different checkpoint until `tf daemon stop` is used. For tests, `TERFINDER_IDLE_TIMEOUT` and `TERFINDER_MODEL` override the config when starting a daemon. Confirmation cannot be disabled through config.

```sh
tf покажи, что лежит в Downloads
tf напомни, в какой папке сейчас терминал
tf покажи, что написано в README.md
tf сделай копию a.txt в Documents
tf мне сейчас нужен Safari, покажи его
tf what has grabbed TCP port 8765
tf how bloated is Downloads
tf what kind of Mac am I running
tf какой процесс занимает больше всего памяти
tf включи AirDrop
tf выключи Stage Manager
tf сделай curl запрос на 'https://example.com?a=1&b=2'
tf ¿cuánta batería me queda?
```

HTTP GET requests honor curl's standard `https_proxy`, lowercase `http_proxy`, `ALL_PROXY`, and `NO_PROXY` environment variables. Proxy selection is intentionally not accepted as free-form curl flags:

```sh
env HTTPS_PROXY=http://127.0.0.1:8080 tf send a request to https://example.com
env ALL_PROXY=socks5h://127.0.0.1:1080 tf fetch example.com
```

## Phrase knowledge base

TerFinder ships with a hashed multilingual term index plus parameterized, anchored phrase rules. Short synonyms use a longest-match lookup instead of scanning every known sentence. Run `tf knowledge` to see the bundled term/rule counts, the local counts, and validate the optional local file.

Add verbs or short phrases under one of the canonical groups `open`, `delete`, `find`, `move`, `rename`, `clear`, `kill`, `size`, `list`, `copy`, `read`, `mkdir`, or `cd`. Prefixes allow the same terms to work after polite openings. Rules handle sentence shapes that cannot be expressed as a leading synonym:

```json
{
  "version": 1,
  "prefixes": ["please", "could you", "kindly"],
  "terms": {
    "open": ["подними", "activate"],
    "find": ["разыщи", "track down"],
    "mkdir": ["заведи папку", "make a folder"]
  },
  "rules": [
    { "match": "сотри кеш {target}", "rewrite": "clear cache {target}" },
    { "match": "где я оказался", "rewrite": "pwd" }
  ]
}
```

Term groups and rules ignore capitalization and surrounding punctuation. Rules match the complete request and apply at most once. Neither mechanism can invoke a shell: the rewritten text still has to map to a registered typed action, target checks, risk classification, and confirmation. `tf remember` and `knowledge.json` are maintainer overrides, not part of the normal user flow.

Running `tf` with no arguments starts an interactive prompt. After viewing a port listener, `kill it` prepares a confirmed SIGTERM action. After finding apps, `first` / `second` selects one for `delete it`. Type `exit` or press Ctrl-D to leave. Context is only held in memory for that session.

## Actions and safety

The current Rust executors locate, open, and quit apps, find/open/delete/move/rename/copy files, list and create directories, print file contents, resolve `cd`, open URLs, make bounded HTTP/HTTPS GET requests, calculate directory size, find large files, inspect processes and TCP listeners, send SIGTERM to a PID or a port's single listener, clear named pip/npm/yarn/pnpm/Homebrew caches, show disk usage, battery charge, date, user, hostname, macOS version, uptime, memory, CPU, and local network addresses, and control Bluetooth power, AirDrop receiving, and Stage Manager. HTTP requests accept either a full URL or a normal domain such as `google.com` (automatically upgraded to `https://google.com`), restrict the initial request and redirects to HTTP(S), use connection and total timeouts, and cap the response at 5 MiB; flags, request bodies, credentials in URLs, and POST/PUT/PATCH/DELETE are refused. Turning AirDrop on also enables Wi-Fi and Bluetooth when needed; a plain `turn on AirDrop` preserves the current receiving audience, or defaults to Contacts Only when receiving was off. A bare name such as `tf telegram` or `tf .` opens the matching app, folder, file, or URL. App lookup tolerates spoken aliases and one-typo names, so `open Telegram`, `launch telegram`, and `open settings` resolve to real bundles; an ambiguous match asks which one. Quitting an app sends SIGTERM to every process inside its bundle after each PID is rechecked, and is treated as a destructive action. App and file deletion moves the item to `~/.Trash`; it does not perform permanent deletion. Filesystem scans have entry limits. App lookup is restricted to `/Applications`, `~/Applications`, and `/System/Applications`. File name lookup checks the current directory and the top level of Desktop, Documents, and Downloads.

All mutations display a plan and require confirmation unless `--yes` is given for an ordinary destructive action. `--dry-run` displays the resolved plan without executing it. Ambiguous targets require a selection; dry run refuses to pick one. There is no `sh -c`, `eval`, or arbitrary command executor. Supported cache handlers invoke a fixed program with fixed arguments. PID 0, PID 1, the CLI's own PID, and its parent PID are protected. Broad roots such as `/`, `/Applications`, `/System`, `/Library`, `/Users`, and the home directory cannot be moved to Trash.

File moves use macOS `renamex_np` with `RENAME_EXCL` to avoid overwriting a destination created after preview. Process termination rechecks the target immediately before SIGTERM. Moving to Trash can fail when source and Trash are on different filesystems; nothing is permanently deleted in that case.

A model-backed `--dry-run` requires an already loaded daemon so the dry run cannot start a process or download a model. Ordinary fast-parser dry runs work without it.

`--yes` does not enable permanent deletion. Permanent deletion is not implemented.

## Model path

The model daemon uses `laya_mlx.load("aac6fef/laya-multilingual-mlx", dtype="float16", batch_size=16, cache_prompts=True)` and `agent.predict(text, questions)`, matching the installed Laya-MLX 0.1.0 API. The capability questions are generated from Rust's registry at request time. The Unix socket is `~/.cache/terfinder/laya.sock`, with a private parent directory and socket permissions. Startup output is appended to `~/.cache/terfinder/daemon.log`; if the daemon exits or is not ready within 5 seconds, the CLI reports it immediately with the tail of that log instead of waiting silently. No network listener or telemetry is used. Model downloading is handled by Laya/Hugging Face during first setup/use.

The model selects an action only above the configured 0.70 decision threshold. This number is an abstention threshold, not a calibrated probability. The Rust client extracts port numbers, exact installed app names, and the named home directories Desktop, Documents, and Downloads for the current model fallback. For large-file searches it can preserve video/PDF/image type and a few recency windows. A month means the previous 30 days, and type filtering uses filename extensions rather than media inspection. If it cannot safely extract required entities, it reports the understood action and makes no changes.

The model cache location depends on the local Hugging Face configuration. On the validation Mac, `du -shL` of the cached multilingual snapshot measured 647 MB on 2026-09-25. Inspect your own cache before removing anything; the standard location is `~/.cache/huggingface/hub/models--aac6fef--laya-multilingual-mlx`. It will download again on the next model request.

## Development

```sh
cargo test
python3 -m unittest discover -s daemon -p 'test_*.py'
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
scripts/bench.sh
scripts/qa.sh
python3 scripts/stress_qa.py ./target/release/tf
python3 daemon/eval.py ./target/release/tf --fast-misses-only
python3 daemon/eval.py ./target/release/tf --conversational --summary
```

`scripts/qa.sh` is the reproducible release gate: formatting, Clippy with warnings denied, both Rust binary test targets, Python daemon tests, release build, the semantic fixture, a multilingual CLI smoke matrix, capability-registry size, shell-injection rejection, and the high-volume black-box stress suite. The permanent catalog contains 7,192 distinct user scenarios across all 40 capabilities: multilingual verbs combined with different apps, files, directories, processes, PIDs, ports, caches, URLs, system controls, recursive project-file searches, conversational and polite wrappers, numbered input, common typos, natural system/Git/process questions, Unix commands, safe developer-tool commands, negations, ambiguous wording, prose false positives, and adversarial input. Each scenario runs once and validates its action plus typed slots; a separately reported 5,000-launch loop tests endurance and is not counted as scenario coverage. Real local HTTP redirects/protocol limits/response caps, mutation dry-runs, and before/after system-state checks are also included. The benchmark script reports measurements from the local Mac. They are not product guarantees. `--fast-only --interpret-only` and `--interpret-only` expose the two routing layers for evaluation without executing actions.

## Current limitations

- Multi-step port inspection plus termination, termination plus verification, app lookup plus removal, and opening hosts then an app are implemented. Every step is preflighted before execution. General multi-step planning is not.
- Interactive context covers port and app references shown above; more general pronouns and follow-up reasoning are not implemented.
- Natural language is unbounded. The combined parser is tested against command-shaped, conversational, multilingual, negated, ambiguous, and adversarial requests, but it will still abstain when confidence is insufficient. It never expands beyond capabilities in the typed registry.
- The local semantic model uses roughly 647 MB on disk and allocates roughly 644 MB while loaded on the validation Mac. Common requests stay on the native path; the daemon exits after its idle timeout.
- Trash moves use exclusive `renamex_np`, so moving an item from a different filesystem can fail cleanly.
- Recursive name/extension search is bounded to the requested project or directory; TerFinder does not perform an implicit full-volume scan.
- The CLI does not support privileged app removal, permanent deletion, default-browser changes, arbitrary downloads, or arbitrary shell commands.

MIT licensed.
