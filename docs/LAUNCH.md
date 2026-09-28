# X launch kit

## Main post

Your terminal shouldn't require memorizing commands.

I built TerFinder: a local natural-language CLI for macOS. Russian, English + 15 languages, typed actions, confirmations, and no AI-generated shell.

7,192 tested scenarios. Demo + repo ↓

## Short alternative

I built a terminal for macOS that understands intent instead of command syntax.

`tf почему мак тормозит`
`tf who owns port 3000`
`tf turn Bluetooth off`

Local model. Typed actions. Confirmation before changes. No AI-generated shell.

## Posting note

Append the repository URL to either version. X counts a normal URL as 23 characters; the main post plus one URL remains under 280 characters.

## Demo recording

Record a clean 20–25 second terminal window. Increase the font size, hide unrelated tabs, and run these in order:

```sh
tf почему мак тормозит
tf кому принадлежит PID 46272
tf найди все .tsx файлы в этом проекте
tf что изменилось в репозитории
tf plan выключи блютуз
```

Use `tf plan` for the Bluetooth shot so the recording cannot change system state. Replace PID `46272` with a real PID from the first command if that process is no longer running. Keep the repository URL visible in the final frame.

## Proof points for replies

- 40 typed actions; the model selects an action but never writes a shell command.
- 7,192 unique black-box scenarios and 36,976 assertions with zero release-gate failures.
- Russian, English, Ukrainian, Spanish, German, French, Portuguese, Italian, Polish, Turkish, Dutch, Czech, Chinese, Japanese, Korean, Arabic, and Hindi.
- Common requests use the native Rust parser; semantic fallback runs locally and stops after inactivity.
- Destructive actions show a resolved plan and require confirmation.
- macOS only for the current public beta.
