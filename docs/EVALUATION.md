# Validation snapshot — 2026-09-28

## 0.6.0 public beta polish — 2026-09-28

The public beta adds a one-command installer, a private managed Python runtime for semantic fallback, automatic discovery of that runtime, an intuitive `tf model` alias, non-erroring stopped-model status, bilingual first-run help, package metadata, a changelog, launch guidance, proxy documentation, and repository hygiene for local development artifacts. The safety and action boundaries are unchanged.

The 0.6.0 release gate retains all 40 typed capabilities, 249 native fixtures, 38 conversational model cases, 76 Rust tests for each binary in debug and release mode, eight Python daemon tests, 7,192 unique black-box scenarios, and the separate 5,000-run endurance loop.

## 0.5.3 natural terminal regressions — 2026-09-27

The terminal transcripts supplied after the first stress expansion are now permanent regressions. Natural requests cover largest files and full-word size units, locating an app "on this Mac", related processes, recent Git commits/current branch/changed files, current-directory size, numbered-list prefixes, recursive project searches by name or extension, disk-space investigation, slow-Mac diagnostics, running browsers, and PID lookup. `что за PID 46272` and `что за процесс 46272` now resolve a process by PID and can no longer fall into port lookup.

Recursive project search is a new bounded read-only `FIND_FILES` capability. It skips build/cache trees such as `.git`, `.venv`, `target`, `node_modules`, `.next`, `dist`, and `build`, limits traversal and result counts, and accepts either a case-insensitive name fragment or extension. Natural Git requests map only to the existing fixed read-only argv allowlist; no shell text is generated.

The question and conversation expansion added polite/colloquial wrappers (`эй tf`, `слушай`, `будь добр`, `hey tf`), numbered input, common Russian typos, natural system-state questions, Git questions without mandatory command syntax, recursive search questions, PID ownership/details, browser-state questions, and slow-Mac diagnostics. The first expanded run exposed 102 distinct broken phrases; the generalized fixes closed all of them without weakening the command/port/PID safety distinctions.

The complete gate passed with 40 capabilities, 249/249 native fixtures, 38/38 conversational cases (24 native and 14 model fallbacks), 76 Rust tests for each of two binaries in debug and release mode, eight Python tests, formatting, and Clippy with warnings denied. The release binary passed 7,192 unique scenarios and 36,976 assertions/checks with zero failures; the separately reported 5,000-launch endurance loop is not counted as unique scenario coverage.

## 0.5.3 scheme-less web requests — 2026-09-26

Natural requests no longer require users to spell a URL scheme. `отправь запрос на google.com` and `отправь запрос curl на google.com` both map directly to `FETCH_URL` with the target normalized to `https://google.com`. Full HTTP(S) URLs and domain-plus-path targets remain supported, while the same protocol, method, flag, timeout, redirect, and response-size restrictions still apply.

At that point the permanent black-box stress gate contained 5,816 unique user scenarios in 37 categories and performed 30,787 assertions/checks against the installed release binary. It covered every then-registered capability with multilingual wording and different concrete apps, files, directories, processes, PIDs, ports, caches, URLs, system-control spellings, Unix commands, allowlisted developer tools, explicit negation, ambiguous requests, narrative false positives, and adversarial input. Each catalog phrase was unique, ran once, and validated both its action and all expected typed slots. A separately reported 5,000-launch endurance loop was not counted as scenario coverage.

The expanded scenarios exposed and fixed additional gaps that smaller suites had missed: `AirDrop only for contacts`, `free space`, natural disk/battery/date/hostname questions, non-allowlisted `git`/`cargo` commands falling through into file interpretation, vague `убей процесс` and `освободи порт` requests being mistaken for application names, and narrative sentences about files or ports being treated as commands. Earlier stress passes also fixed English Bluetooth toggles with the object between `turn` and `on/off`, normalized English contractions in negated requests, Korean particles attached to domains, colloquial `дерни <domain>` requests, and plural `show running processes` being intercepted as a lookup for a process named `running`.

The parser regression suite now has 74 Rust tests for each of the two binaries. The previous complete gate also passed 218/218 native fixtures, 38/38 conversational cases with 17 model fallbacks, eight Python tests, formatting, and Clippy with warnings denied. There were zero wrong mappings, unsafe mappings, or abstentions in the conversational set.

## 0.5.2 flexible toggles and bounded HTTP GET — 2026-09-26

System toggles now extract a trailing state from natural English word order, including `turn the StageManager on`, instead of requiring adjacent `turn on`. The reversible Stage Manager toggle executes immediately after showing its plan. A new `FETCH_URL` action understands requests such as `сделай curl запрос на <URL>` and sends a fixed, body-free HTTP(S) GET without a shell. The executor restricts both the initial protocol and redirects to HTTP(S), applies connection/total timeouts and a 5 MiB response cap, and rejects credentials, non-HTTP schemes, curl flags, request bodies, and mutating HTTP methods.

The complete gate passed with 39 capabilities, 216/216 native fixtures, 38/38 conversational fixtures, 72 Rust tests for each binary in debug and release mode, eight Python tests, and zero wrong or unsafe mappings.

## 0.5.1 macOS controls — 2026-09-26

AirDrop and Stage Manager are now first-class typed system actions instead of falling through to application lookup. `включи AirDrop` preserves an existing Contacts Only or Everyone audience, defaults to Contacts Only when receiving was off, and plans any required Wi-Fi or Bluetooth activation before changing the receiving mode. Explicit contacts/everyone/off requests remain available. Stage Manager reads and changes the live per-user macOS setting. All mutations keep the normal preview and confirmation boundary.

The regression fixture adds Russian and English AirDrop and Stage Manager cases, the CLI smoke matrix covers compact `StageManager` spelling, and Wi-Fi hardware-port parsing has a unit test. The complete gate passed with 38 capabilities, 214/214 native fixtures, 38/38 conversational fixtures (including 17 model fallbacks), 70 Rust tests for each of two binaries in debug and release mode, eight Python tests, and zero wrong or unsafe mappings. Live validation also confirmed Bluetooth on, Wi-Fi on, AirDrop receiving set to Everyone, and Stage Manager on after executing the installed binary.

## 0.5.0 natural-language-first gate — 2026-09-26

The normal interface no longer asks users to learn `capabilities`, `remember`, or exact command-shaped phrases. Common wording stays on the native Rust path; unfamiliar wording automatically reaches the local multilingual classifier. The model sees natural outcome labels, first chooses a domain, and can return only an action from the Rust capability registry. Entity values are extracted and validated in Rust rather than generated as shell text.

A separate conversational fixture intentionally uses indirect wording such as `I feel like using Safari right now`, `How long has this laptop been awake?`, `How bloated is Downloads?`, and natural requests in Russian, French, Spanish, German, Italian, Polish, Turkish, Dutch, Czech, Japanese, Korean, Arabic, and Hindi. On the local Apple Silicon validation machine:

| Conversational check | Result |
| --- | ---: |
| Cases | 38 |
| Native parser correct | 21 |
| Native misses routed to the model | 17 |
| Combined correct | 38 / 38 |
| Wrong mappings | 0 |
| Wrong destructive mappings | 0 |
| Abstentions | 0 |
| Warm fixture time | 1.17 s |

Negative probes cover explicit negation, irrelevant statements mentioning apps/files/folders, incidental port numbers, and phrases that mention stopping without requesting it. Explicitly negated actions are refused before semantic fallback. The release gate currently includes 69 Rust tests for each of two binaries plus eight Python daemon tests. This is evidence for the tested capability surface, not a claim that every possible sentence can be understood.

## 0.4.2 natural process ranking — 2026-09-26

Requests such as `какой процесс занимает больше всего памяти`, `что жрет оперативку`, `which process uses the most memory`, and `top CPU processes` now map by semantic markers rather than exact sentences. `LIST_PROCESSES` now returns a real table sorted by resident memory or CPU, with PID, CPU percentage, memory percentage, RSS, and a readable process name. Unknown requests no longer tell the user to invent a command immediately; they fail safely and are recorded automatically for review.

## 0.4.1 Bluetooth routing fix — 2026-09-26

Bluetooth power requests now take precedence over the generic app-quit verb. The deterministic parser recognizes `блютуз`, the common variants `блютус`, `блутуз`, and `блутус`, one-edit English misspellings such as `bluetoth`, and the split spelling `blue tooth`. Regression fixtures verify that these requests produce `SET_BLUETOOTH_POWER` rather than `QUIT_APP`; actual power changes remain confirmation-gated, and the test suite only uses interpretation and dry-run modes.

## 0.4.0 release gate — 2026-09-26

The CLI now exposes 36 typed capabilities and five new read-only macOS diagnostics: system information, uptime, memory, CPU, and local network addresses. All capability examples are executable parser tests, and every registered capability must have an end-to-end semantic fixture. The parser also has multilingual two-path extraction tests for copy, move, and rename across 17 language families where the bundled action exists.

`scripts/qa.sh` passed in full on an Apple M4 Mac running macOS 15.7.5:

| Check | Result |
| --- | ---: |
| Typed capabilities | 36 |
| Semantic fixture | 208 / 208 correct fast mappings |
| Wrong mappings in fixture | 0 |
| Fast-parser misses in fixture | 0 |
| Bundled unique terms | 518 |
| Anchored phrase rules | 101 |
| Diagnostic-language phrases | 119 |
| Rust tests | 66 passed for each of 2 binary targets, in debug and release modes |
| Python daemon tests | 8 passed |
| Clippy | passed with warnings denied |
| Release build | passed |
| Stripped release binary | 1,017,344 bytes |
| `--version`, 100 launches | 0.19 s total |
| Parser-only port case, 20 launches | 0.60 s total |
| Parser-only maximum RSS | 1,343,488 bytes |

In addition to the curated fixture, invariant tests exercise all 518 bundled terms, all 101 phrase rules, uppercase and irregular-whitespace variants of all 208 fixtures, every documented capability example, 47 multilingual copy/move/rename cases, vague destructive requests, protected paths, non-overwriting file operations, PID guards, and shell-metacharacter attempts against allowlisted developer tools. These checks cover the supported grammar and safety boundaries; they are not a claim that every possible natural-language sentence is understood.

## Multilingual phrase knowledge update — 2026-09-26

The native phrase layer now contains 518 unique synonym terms, 35 politeness prefixes, and 101 anchored sentence rules. Terms use a hash index with longest-phrase matching; sentence rules are indexed by their first token. The bundled languages are Russian, English, Ukrainian, Spanish, German, French, Portuguese, Italian, Polish, Turkish, Dutch, Czech, Chinese, Japanese, Korean, Arabic, and Hindi.

At that point the expanded fixture contained 193 cases. `python3 daemon/eval.py ./target/release/tf --fast-misses-only` reported 193 correct fast-parser mappings and zero misses. `cargo test`, `cargo clippy --all-targets -- -D warnings`, the release build, and all eight daemon unit tests passed. A local sample of 200 separate release-binary launches parsing `please fire up Safari` took 0.39 seconds total, about 1.95 ms per launch including process startup. This timing is a local measurement, not a product guarantee.

The earlier snapshot follows for historical comparison.

0.2.0 added Unix argv commands, directory listing/copy/mkdir/cat/cd, and non-Russian verbs to the fast parser. The 2026-09-25 table below is the historical 0.1.0 fixture run (124 cases); at that time the model was opt-in (`autostart = false`). Re-run `cargo test` and `scripts/bench.sh` on the current tree for current results.

Environment: Apple Silicon MacBook Air, macOS 15.6, Rust 1.98.1. Fast-path rows were re-measured on 2026-09-25 after the parser rewrite; model-backed rows come from an earlier session that had Laya-MLX 0.1.0 and the multilingual MLX checkpoint installed. The evaluation used `daemon/eval.py`, `tests/fixtures/intents.json`, and the release `tf` binary. Fast-path rows were produced with `--fast-only --interpret-only`; model rows used `--interpret-only` against a local Metal-backed daemon. No requested action was executed.

| Measure | Observed |
| --- | ---: |
| Fixture cases | 124 |
| Correct fast-parser mappings | 124 |
| Correct combined mappings | 124 |
| Wrong typed action mappings | 0 |
| Wrong destructive action mappings | 0 |
| Fast-parser abstentions on fixtures | 0 |
| Stripped release binary size (`tf`) | 587,792 bytes |
| `--version`, 100 launches | 0.18 s total, about 1.8 ms/launch |
| `port 8765`, 20 launches | 0.53 s total, about 26 ms/launch including `lsof` |
| `--version` maximum RSS | 1,425,408 bytes |
| Cold model request | 2.64 s (earlier run) |
| Warm model request | 0.05 s (earlier run) |
| Loaded daemon MLX active allocation | 643,832,354 bytes (earlier run) |
| Loaded daemon MLX free cache | 112,347,289 bytes (earlier run) |
| Loaded daemon MLX peak | 756,177,823 bytes (earlier run) |

The timing samples are local measurements, not guaranteed latency. The cold and warm model examples were “I would like to remove Google Chrome” with the older unfiltered registry prompt; both abstained. After filtering candidates by real app names, that phrase correctly maps to `REMOVE_APP` with the installed Google Chrome target. No GPU wattage was measured. The model snapshot measured 647 MB with `du -shL`; symlink-aware measurement matters for the Hugging Face cache.

After stopping the model daemon, `tf port 8765` completed and `tf daemon status` still reported no daemon. With a 60-second test timeout, a loaded daemon was later absent from both its socket and process table after the idle period. The model memory was thereby released with process exit. No wattage estimate was made.

The Laya semantic route is not yet fit for the product's broad-language requirement. A grouped choice question missed colloquial port wording. Per-capability yes/no questions produced many high false positives. A two-stage domain/action question abstained on Chrome removal until Rust shortlisted app capabilities using the actual installed app name. Shortlisting directory inspection capabilities recovered one Russian request for large videos in Downloads; its typed plan includes a video-extension filter and a 30-day modified-time filter. The current implementation keeps the two-stage conservative route to prevent unintended actions. This is a functional limitation, not a success claim for universal understanding.

Fast parser fixtures include Russian, English, slang, typos, mixed language, apps, files, processes, ports, caches, disk usage, unsupported goals, multi-step goals, and the newly added launch/quit and URL phrasings. The current run reports 124 correct typed interpretations, no abstentions, and no wrong action mappings; unsupported goals still return `UNSUPPORTED` instead of an executable intent. The evaluator checks expected constraint fields when present, not just the action label.

The combined column matches the fast column because every fixture now resolves before the model is contacted: the whole 124-case run took 0.58 seconds with zero abstentions. The model route itself was not exercised in this run, since the available Python environment has no `mlx` and the daemon reports `ModuleNotFoundError` for model requests. Re-run `TERFINDER_PYTHON="$PWD/daemon/.venv/bin/python" daemon/.venv/bin/python daemon/eval.py` in an environment with `laya-mlx==0.1.0` installed to measure the model route on wording outside the fixture set. Earlier model-backed measurements are kept as historical context and are not claims about the current build.

The launcher now fails fast: when the daemon exits at import time the CLI reports it in well under a second with the tail of `~/.cache/terfinder/daemon.log`, instead of polling for five seconds.
