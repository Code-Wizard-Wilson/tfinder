"""Evaluate the current Rust CLI and local Laya route against the intent fixture."""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = "conversational.json" if "--conversational" in sys.argv else "intents.json"
CASES = json.loads((ROOT / "tests/fixtures" / FIXTURE).read_text())
BINARY = Path(sys.argv[1]) if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else ROOT / "target/release/tf"
DESTRUCTIVE = {"REMOVE_APP", "DELETE_FILE", "DELETE_DIRECTORY", "MOVE_FILE", "RENAME_FILE",
               "KILL_PROCESS", "KILL_PORT_PROCESS", "CLEAR_CACHE", "QUIT_APP"}


def interpret(text: str, fast: bool):
    args = [str(BINARY), "--interpret-only"]
    if fast:
        args.append("--fast-only")
    result = subprocess.run([*args, text], capture_output=True, text=True, timeout=240)
    if result.returncode:
        return None, result.stderr.strip()
    try:
        parsed = json.loads(result.stdout)
        return parsed, None
    except (ValueError, KeyError) as exc:
        return None, str(exc)


def main():
    fast_success = 0
    fast_misses = []
    combined_success = 0
    false_mappings = []
    unsafe_false_mappings = []
    abstentions = []
    started = time.perf_counter()
    for case in CASES:
        text, expected = case["input"], case["expected_goal"]
        expected_fields = case.get("expected_fields", {})
        fast, _ = interpret(text, True)
        fast_action = "MULTI_STEP" if fast and "steps" in fast else fast.get("action") if fast else None
        fast_matches = fast_action == expected and all(fast.get(key) == value for key, value in expected_fields.items())
        if fast_matches:
            fast_success += 1
        else:
            fast_misses.append(text)
        if "--fast-misses-only" in sys.argv:
            continue
        combined, error = interpret(text, False)
        combined_action = "MULTI_STEP" if combined and "steps" in combined else combined.get("action") if combined else None
        combined_matches = combined_action == expected and all(combined.get(key) == value for key, value in expected_fields.items())
        if combined_matches:
            combined_success += 1
        elif combined is None:
            abstentions.append({"input": text, "expected": expected, "reason": error})
        else:
            mismatch = {"input": text, "expected": expected, "actual": combined_action}
            if expected_fields:
                mismatch["expected_fields"] = expected_fields
                mismatch["actual_fields"] = {key: combined.get(key) for key in expected_fields}
            false_mappings.append(mismatch)
            if combined_action in DESTRUCTIVE:
                unsafe_false_mappings.append(mismatch)
    report = {
        "cases": len(CASES),
        "fast_correct": fast_success,
        "fast_misses": fast_misses,
        "combined_correct": combined_success,
        "false_mappings": false_mappings,
        "unsafe_false_mappings": unsafe_false_mappings,
        "abstentions": abstentions,
        "elapsed_seconds": round(time.perf_counter() - started, 2),
    }
    if "--fast-misses-only" in sys.argv:
        print(json.dumps({"cases": len(CASES), "fast_correct": fast_success,
                          "fast_misses": fast_misses}, ensure_ascii=False, indent=2))
        if "--require-perfect" in sys.argv and fast_misses:
            raise SystemExit(1)
        return
    if "--summary" in sys.argv:
        report["fast_misses"] = len(fast_misses)
        report["false_mappings"] = len(false_mappings)
        report["unsafe_false_mappings"] = len(unsafe_false_mappings)
        report["abstentions"] = len(abstentions)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if ("--require-perfect" in sys.argv and
            (combined_success != len(CASES) or false_mappings or
             unsafe_false_mappings or abstentions)):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
