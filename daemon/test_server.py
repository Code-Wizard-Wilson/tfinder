import unittest
import json
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from server import classify, questions, unavailable_anchor


CAPABILITIES = [
    {"name": "REMOVE_APP", "description": "Move app to Trash", "examples": ["удали Chrome"]},
    {"name": "FIND_PORT_PROCESS", "description": "Find TCP listener", "examples": ["кто на 8765"]},
]


class FakeAgent:
    def __init__(self, responses):
        self.responses = iter(responses)
        self.questions = None

    def predict(self, text, qs):
        self.questions = qs
        return next(self.responses)


class ServerTests(unittest.TestCase):
    def test_registry_drives_questions(self):
        self.assertIn("domain", questions(CAPABILITIES))
        self.assertIn("uninstall", questions(CAPABILITIES, "APP")["action"]["criteria"])
        self.assertIn("inspect port", questions(CAPABILITIES, "PORT")["action"]["criteria"])

    def test_typed_choice(self):
        fake = FakeAgent([
            {"answers": {"domain": {"choice": "applications", "probabilities": {"applications": 0.90}}}},
            {"answers": {"action": {"choice": "uninstall", "probabilities": {"uninstall": 0.92}}}},
        ])
        self.assertEqual(classify({"text": "снеси Chrome", "capabilities": CAPABILITIES}, fake),
                         {"action": "REMOVE_APP", "confidence": 0.92})

    def test_unsupported(self):
        fake = FakeAgent([
            {"answers": {"domain": {"choice": "other", "probabilities": {"other": 0.95}}}},
            {"answers": {"goal": {"choice": "OTHER", "probabilities": {"OTHER": 0.99}}}},
        ])
        self.assertEqual(classify({"text": "скачай Ubuntu", "capabilities": CAPABILITIES}, fake)["action"], "UNSUPPORTED")

    def test_unsupported_but_understood(self):
        fake = FakeAgent([
            {"answers": {"domain": {"choice": "other", "probabilities": {"other": 0.95}}}},
            {"answers": {"goal": {"choice": "DOWNLOAD_FILE", "probabilities": {"DOWNLOAD_FILE": 0.99}}}},
        ])
        result = classify({"text": "download Ubuntu ISO", "capabilities": CAPABILITIES}, fake)
        self.assertEqual(result["semantic_goal"], "DOWNLOAD_FILE")
        self.assertFalse(unavailable_anchor("Downloads takes too much space", "DOWNLOAD_FILE"))

    def test_strong_unsupported_install_anchor(self):
        fake = FakeAgent([
            {"answers": {"domain": {"choice": "other", "probabilities": {"other": 0.95}}}},
        ])
        result = classify({"text": "поставь мне Docker", "capabilities": CAPABILITIES}, fake)
        self.assertEqual(result["action"], "UNSUPPORTED")
        self.assertEqual(result["semantic_goal"], "INSTALL_APP")
        self.assertEqual(result["semantic_confidence"], 1.0)

    def test_malformed_answer_abstains(self):
        fake = FakeAgent([{"answers": {"domain": {"choice": "applications", "probabilities": {"applications": True}}}}])
        self.assertEqual(classify({"text": "удали Chrome", "capabilities": CAPABILITIES}, fake)["action"], "UNCLEAR")

    def test_single_domain_skips_domain_question(self):
        fake = FakeAgent([{"answers": {"action": {
            "choice": "uninstall", "probabilities": {"uninstall": 0.97}
        }}}])
        result = classify({"text": "remove Chrome", "capabilities": CAPABILITIES[:1]}, fake)
        self.assertEqual(result["action"], "REMOVE_APP")

    def test_concurrent_daemon_start_keeps_one_socket(self):
        with tempfile.TemporaryDirectory(prefix="tf-daemon-", dir="/tmp") as directory:
            path = Path(directory) / "laya.sock"
            script = Path(__file__).with_name("server.py")
            processes = [subprocess.Popen([sys.executable, str(script), str(path)],
                                          stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                          stderr=subprocess.PIPE) for _ in range(2)]
            try:
                response = None
                for _ in range(100):
                    try:
                        with socket.socket(socket.AF_UNIX) as client:
                            client.connect(str(path))
                            client.sendall(b'{"control":"status"}\n')
                            response = json.loads(client.recv(4096))
                        break
                    except (OSError, ValueError):
                        time.sleep(0.01)
                details = [(p.poll(), p.stderr.read().decode() if p.poll() is not None else "running") for p in processes]
                if response is None and any("Operation not permitted" in error for _, error in details):
                    self.skipTest("sandbox denies Unix socket bind")
                self.assertIsNotNone(response, details)
                self.assertEqual(response["ok"], True)
                with socket.socket(socket.AF_UNIX) as client:
                    client.connect(str(path))
                    client.sendall(b'{"control":"stop"}\n')
                    self.assertTrue(json.loads(client.recv(4096))["stopping"])
                for process in processes:
                    process.wait(timeout=3)
                self.assertFalse(path.exists())
            finally:
                for process in processes:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=3)
                    process.stderr.close()


if __name__ == "__main__":
    unittest.main()
