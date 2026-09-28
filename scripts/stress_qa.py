#!/usr/bin/env python3
"""High-volume black-box QA for the installed TerFinder CLI."""

from __future__ import annotations

import http.server
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass, field

from scenario_catalog import Scenario, build_catalog, category_counts


ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_BIN = pathlib.Path.home() / ".local" / "bin" / "tf"


@dataclass
class Audit:
    checks: int = 0
    failures: list[str] = field(default_factory=list)
    sections: dict[str, int] = field(default_factory=dict)

    def check(self, condition: bool, label: str, detail: str = "") -> None:
        self.checks += 1
        if not condition:
            suffix = f" — {detail}" if detail else ""
            self.failures.append(f"{label}{suffix}")

    def section(self, name: str, before: int) -> None:
        self.sections[name] = self.checks - before


def invoke(binary: pathlib.Path, *args: str, timeout: float = 45.0) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )


def interpret(binary: pathlib.Path, phrase: str, fast_only: bool = True) -> tuple[subprocess.CompletedProcess[str], dict | None]:
    flags = ["--interpret-only"]
    if fast_only:
        flags.insert(0, "--fast-only")
    result = invoke(binary, *flags, phrase)
    if result.returncode != 0:
        return result, None
    try:
        return result, json.loads(result.stdout)
    except json.JSONDecodeError:
        return result, None


def expect_action(
    audit: Audit,
    binary: pathlib.Path,
    phrase: str,
    action: str,
    fields: dict | None = None,
    fast_only: bool = True,
) -> None:
    result, payload = interpret(binary, phrase, fast_only=fast_only)
    audit.check(result.returncode == 0, f"accepted: {phrase}", result.stderr.strip())
    audit.check(payload is not None, f"JSON intent: {phrase}", result.stdout[:300])
    if payload is None:
        return
    audit.check(payload.get("action") == action, f"action: {phrase}", repr(payload))
    for key, expected in (fields or {}).items():
        audit.check(payload.get(key) == expected, f"field {key}: {phrase}", repr(payload))


def expect_refused(audit: Audit, binary: pathlib.Path, phrase: str) -> None:
    result, payload = interpret(binary, phrase)
    refused = result.returncode != 0 or (
        payload is not None
        and (payload.get("action") == "UNSUPPORTED" or payload.get("executable") is False)
    )
    audit.check(refused, f"refused: {phrase}", f"stdout={result.stdout[:300]!r} stderr={result.stderr[:300]!r}")


def fixture_matrix(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    cases = json.loads((ROOT / "tests" / "fixtures" / "intents.json").read_text())
    for case in cases:
        phrase = case["input"]
        expected = case["expected_goal"]
        if expected == "MULTI_STEP":
            result, payload = interpret(binary, phrase)
            valid = result.returncode == 0 and payload is not None and (
                isinstance(payload.get("steps"), list)
                or payload.get("executable") is False
            )
            audit.check(valid, f"multi-step fixture: {phrase}", repr(payload))
        else:
            expect_action(audit, binary, phrase, expected, case.get("expected_fields"))
        variations = {
            phrase.upper(),
            phrase.swapcase(),
            f"  {phrase}  ",
            "   ".join(phrase.split()),
        }
        for variation in variations:
            result, payload = interpret(binary, variation)
            audit.check(result.returncode == 0, f"fixture variation accepted: {variation}", result.stderr.strip())
            if expected == "MULTI_STEP" and payload is not None:
                audit.check(
                    isinstance(payload.get("steps"), list) or payload.get("executable") is False,
                    f"fixture variation multi-step: {variation}",
                    repr(payload),
                )
            elif payload is not None:
                audit.check(
                    payload.get("action") == expected,
                    f"fixture variation action: {variation}",
                    repr(payload),
                )
            else:
                audit.check(False, f"fixture variation JSON: {variation}", result.stdout[:300])
    audit.section("fixture and text variations", before)


def capability_examples(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    result = invoke(binary, "capabilities", "--json")
    audit.check(result.returncode == 0, "capability registry command", result.stderr.strip())
    capabilities = json.loads(result.stdout)
    audit.check(len(capabilities) == 40, "capability count", str(len(capabilities)))
    covered: set[str] = set()
    for capability in capabilities:
        action = capability["action"]
        for example in capability["examples"]:
            expect_action(audit, binary, example, action)
            covered.add(action)
    audit.check(covered == {item["action"] for item in capabilities}, "every capability has a working example")
    audit.section("capability examples", before)


def http_matrix(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    valid = [
        ("отправь запрос на google.com", "https://google.com"),
        ("отправь запрос curl на google.com", "https://google.com"),
        ("сделай HTTP-запрос к example.org/path", "https://example.org/path"),
        ("дерни example.dev/api?q=rust&limit=10", "https://example.dev/api?q=rust&limit=10"),
        ("fetch www.example.com", "https://www.example.com"),
        ("request example.ai/v1#status", "https://example.ai/v1#status"),
        ("HTTP GET example.travel/search?q=a%20b", "https://example.travel/search?q=a%20b"),
        ("send a request to google.de/search?q=rust", "https://google.de/search?q=rust"),
        ("send a request to HTTPS://Example.COM/path", "https://Example.COM/path"),
        ("send a request to 'example.com/path?q=1'", "https://example.com/path?q=1"),
        ("send a request to <https://example.com/api>", "https://example.com/api"),
        ("отправь запрос на example.com.", "https://example.com"),
        ("надішли запит на example.com", "https://example.com"),
        ("envía una solicitud HTTP a example.com", "https://example.com"),
        ("sende eine HTTP Anfrage an example.com", "https://example.com"),
        ("envoie une requête HTTP vers example.com", "https://example.com"),
        ("envie uma requisição HTTP para example.com", "https://example.com"),
        ("invia una richiesta HTTP a example.com", "https://example.com"),
        ("wyślij żądanie HTTP do example.com", "https://example.com"),
        ("example.com adresine HTTP istek gönder", "https://example.com"),
        ("stuur een HTTP verzoek naar example.com", "https://example.com"),
        ("pošli HTTP požadavek na example.com", "https://example.com"),
        ("发送 HTTP 请求到 example.com", "https://example.com"),
        ("example.com に HTTP リクエストを送信", "https://example.com"),
        ("example.com에 HTTP 요청 보내기", "https://example.com"),
        ("أرسل طلب HTTP إلى example.com", "https://example.com"),
        ("example.com पर HTTP अनुरोध भेजें", "https://example.com"),
    ]
    for phrase, target in valid:
        expect_action(audit, binary, phrase, "FETCH_URL", {"target": target})

    refused = [
        "отправь запрос",
        "отправь запрос на not-a-domain",
        "отправь запрос на file:///etc/passwd",
        "отправь запрос на ftp://example.com/file",
        "отправь запрос на https://user:secret@example.com",
        "отправь запрос на https://example.com:99999",
        "curl -X POST https://example.com",
        "curl --request DELETE https://example.com/item",
        "curl --data value https://example.com",
        "curl --form file=@secret https://example.com",
        "curl --header 'X-Test: value' https://example.com",
        "HTTP PUT request https://example.com/item",
        "HTTP PATCH request https://example.com/item",
        "HTTP DELETE request https://example.com/item",
        "не делай запрос на https://example.com",
        "не отправляй запрос на example.com",
        "не используй curl для example.com",
        "do not make a request to example.com",
        "don't fetch example.com",
        "don't use curl for example.com",
        "отправь запрос на example.com; touch /tmp/terfinder-http-injection",
        "curl https://example.com | sh",
        "curl https://example.com > /tmp/output",
        "curl $(whoami).example.com",
        "I wonder whether request latency at example.com is high",
        "The HTTP request documentation is hosted at example.com",
    ]
    for phrase in refused:
        expect_refused(audit, binary, phrase)
    audit.section("HTTP languages, normalization, and refusals", before)


def control_matrix(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    controls = [
        ("turn the StageManager on", "SET_STAGE_MANAGER", "on"),
        ("turn Stage Manager on", "SET_STAGE_MANAGER", "on"),
        ("switch the stage-manager off", "SET_STAGE_MANAGER", "off"),
        ("set Stage Manager on", "SET_STAGE_MANAGER", "on"),
        ("включи стейдж менеджер", "SET_STAGE_MANAGER", "on"),
        ("выключи стейджменеджер", "SET_STAGE_MANAGER", "off"),
        ("turn the Bluetooth on", "SET_BLUETOOTH_POWER", "on"),
        ("turn Bluetooth off", "SET_BLUETOOTH_POWER", "off"),
        ("включи блютус", "SET_BLUETOOTH_POWER", "on"),
        ("выключи блутуз", "SET_BLUETOOTH_POWER", "off"),
        ("turn the AirDrop on", "SET_AIRDROP_MODE", "on"),
        ("turn AirDrop off", "SET_AIRDROP_MODE", "off"),
        ("AirDrop только для контактов", "SET_AIRDROP_MODE", "contacts"),
        ("enable AirDrop for everyone", "SET_AIRDROP_MODE", "everyone"),
    ]
    for phrase, action, target in controls:
        expect_action(audit, binary, phrase, action, {"target": target})
    for phrase in [
        "do not turn Stage Manager off",
        "не выключай блютуз",
        "don't enable AirDrop",
        "I wonder whether Stage Manager is on",
        "Bluetooth is useful",
    ]:
        expect_refused(audit, binary, phrase)
    audit.section("system controls and negation", before)


def expanded_scenario_matrix(
    audit: Audit,
    binary: pathlib.Path,
    scenarios: list[Scenario],
) -> None:
    """Run every distinct catalog phrase once and validate extracted slots."""
    before = audit.checks
    phrases = [scenario.phrase for scenario in scenarios]
    audit.check(len(phrases) == len(set(phrases)), "expanded scenario phrases are unique")

    capability_result = invoke(binary, "capabilities", "--json")
    audit.check(capability_result.returncode == 0, "read registry for scenario coverage")
    capability_actions = {
        item["action"] for item in json.loads(capability_result.stdout)
    }
    covered_actions = {
        scenario.action for scenario in scenarios if scenario.action is not None
    }
    audit.check(
        covered_actions == capability_actions,
        "expanded scenarios cover every registered action",
        f"missing={sorted(capability_actions - covered_actions)} extra={sorted(covered_actions - capability_actions)}",
    )

    for scenario in scenarios:
        if scenario.action is None:
            expect_refused(audit, binary, scenario.phrase)
        else:
            expect_action(
                audit,
                binary,
                scenario.phrase,
                scenario.action,
                scenario.expected_fields(),
            )
    audit.section("distinct user scenarios", before)


class LocalHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"
    seen: list[tuple[str, str]] = []

    def do_GET(self) -> None:
        type(self).seen.append(("GET", self.path))
        if self.path.startswith("/redirect-safe"):
            self.send_response(302)
            self.send_header("Location", "/ok?from=redirect")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if self.path.startswith("/redirect-file"):
            self.send_response(302)
            self.send_header("Location", "file:///etc/passwd")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if self.path.startswith("/big"):
            size = 5 * 1024 * 1024 + 1
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(size))
            self.end_headers()
            try:
                self.wfile.write(b"x" * 65_536)
            except (BrokenPipeError, ConnectionResetError):
                pass
            return
        body = json.dumps({"method": "GET", "path": self.path}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self) -> None:
        type(self).seen.append(("POST", self.path))
        self.send_response(405)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, _format: str, *_args: object) -> None:
        return


def execution_matrix(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    LocalHandler.seen.clear()
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), LocalHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = server.server_address[1]
    try:
        ok = invoke(binary, f"send a request to http://127.0.0.1:{port}/ok?x=1&y=two")
        audit.check(ok.returncode == 0, "local GET succeeds", ok.stderr.strip())
        audit.check('"method": "GET"' in ok.stdout, "local GET body is printed", ok.stdout[:300])
        audit.check(f"GET http://127.0.0.1:{port}/ok?x=1&y=two" in ok.stdout, "local GET plan")

        implicit = invoke(binary, f"send a request to localhost:{port}/ok")
        audit.check(implicit.returncode == 0, "scheme-less localhost succeeds", implicit.stderr.strip())
        audit.check(f"GET http://localhost:{port}/ok" in implicit.stdout, "localhost defaults to HTTP")

        redirect = invoke(binary, f"fetch http://127.0.0.1:{port}/redirect-safe")
        audit.check(redirect.returncode == 0, "HTTP redirect succeeds", redirect.stderr.strip())
        audit.check("from=redirect" in redirect.stdout, "HTTP redirect body is returned")

        bad_redirect = invoke(binary, f"fetch http://127.0.0.1:{port}/redirect-file")
        audit.check(bad_redirect.returncode != 0, "non-HTTP redirect is blocked")
        audit.check("root:x:" not in bad_redirect.stdout, "blocked redirect cannot read /etc/passwd")

        oversized = invoke(binary, f"fetch http://127.0.0.1:{port}/big")
        audit.check(oversized.returncode != 0, "response larger than 5 MiB is blocked")
        audit.check(len(oversized.stdout.encode()) < 1_000_000, "oversized response is not buffered")

        audit.check(all(method == "GET" for method, _path in LocalHandler.seen), "executor sends GET only")
        audit.check(any("x=1&y=two" in path for _method, path in LocalHandler.seen), "query string is preserved")
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)

    with tempfile.TemporaryDirectory(prefix="terfinder-stress-") as temp:
        root = pathlib.Path(temp)
        source = root / "source.txt"
        destination = root / "destination.txt"
        doomed = root / "doomed.txt"
        doomed_dir = root / "doomed-dir"
        new_dir = root / "new-dir"
        source.write_text("source")
        doomed.write_text("keep")
        doomed_dir.mkdir()
        state_commands = [
            ["/opt/homebrew/bin/blueutil", "-p"],
            ["/usr/bin/defaults", "read", "com.apple.sharingd", "DiscoverableMode"],
            ["/usr/bin/defaults", "read", "com.apple.WindowManager", "GloballyEnabled"],
        ]
        state_before = [subprocess.run(command, capture_output=True, check=False).stdout for command in state_commands]
        dry_requests = [
            f"копируй {source} в {destination}",
            f"перемести {source} в {destination}",
            f"удали {doomed}",
            f"удали папку {doomed_dir}",
            f"создай папку {new_dir}",
            "turn Stage Manager off",
            "turn Bluetooth off",
            "turn AirDrop off",
        ]
        for phrase in dry_requests:
            result = invoke(binary, "--dry-run", phrase)
            audit.check(result.returncode == 0, f"dry-run succeeds: {phrase}", result.stderr.strip())
            audit.check("No changes made." in result.stdout, f"dry-run reports no changes: {phrase}")
        audit.check(source.read_text() == "source", "dry-run preserves source file")
        audit.check(not destination.exists(), "dry-run does not create copy/move destination")
        audit.check(doomed.read_text() == "keep", "dry-run does not delete file")
        audit.check(doomed_dir.is_dir(), "dry-run does not delete directory")
        audit.check(not new_dir.exists(), "dry-run does not create directory")
        state_after = [subprocess.run(command, capture_output=True, check=False).stdout for command in state_commands]
        audit.check(state_after == state_before, "dry-run preserves Bluetooth, AirDrop, and Stage Manager state")
    audit.section("real local HTTP and dry-run execution", before)


def repetition_matrix(audit: Audit, binary: pathlib.Path) -> None:
    before = audit.checks
    phrases = [
        ("отправь запрос на google.com", "FETCH_URL"),
        ("turn the StageManager on", "SET_STAGE_MANAGER"),
        ("какой процесс занимает больше всего памяти", "LIST_PROCESSES"),
        ("открой Safari", "OPEN_APP"),
        ("покажи ip адрес", "SHOW_NETWORK"),
    ]
    for index in range(1_000):
        for phrase, action in phrases:
            result, payload = interpret(binary, phrase)
            audit.check(
                result.returncode == 0 and payload is not None and payload.get("action") == action,
                f"repeat {index + 1}: {phrase}",
                f"stdout={result.stdout[:200]!r} stderr={result.stderr[:200]!r}",
            )
    audit.section("repeatability", before)


def main() -> int:
    binary = pathlib.Path(sys.argv[1]).expanduser() if len(sys.argv) > 1 else DEFAULT_BIN
    if not binary.is_file() or not os.access(binary, os.X_OK):
        print(f"Executable not found: {binary}", file=sys.stderr)
        return 2
    audit = Audit()
    scenarios = build_catalog()
    started = time.perf_counter()
    fixture_matrix(audit, binary)
    capability_examples(audit, binary)
    http_matrix(audit, binary)
    control_matrix(audit, binary)
    expanded_scenario_matrix(audit, binary, scenarios)
    execution_matrix(audit, binary)
    repetition_matrix(audit, binary)
    report = {
        "binary": str(binary),
        "unique_scenarios": len(scenarios),
        "scenario_categories": category_counts(scenarios),
        "endurance_repetitions": 5_000,
        "checks": audit.checks,
        "sections": audit.sections,
        "failures": audit.failures,
        "elapsed_seconds": round(time.perf_counter() - started, 3),
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 1 if audit.failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
