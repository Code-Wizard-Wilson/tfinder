"""Local Laya-MLX intent classifier. One line of JSON per Unix-socket request."""
from __future__ import annotations

import json
import os
import socket
import sys
import time
import fcntl
from pathlib import Path

MODEL = os.environ.get("TERFINDER_MODEL", "aac6fef/laya-multilingual-mlx")
TIMEOUT = max(60, int(os.environ.get("TERFINDER_IDLE_TIMEOUT", "600")))
_agent = None


def domain(name: str) -> str:
    if "PORT" in name:
        return "PORT"
    if "PROCESS" in name:
        return "PROCESS"
    if "APP" in name:
        return "APP"
    if "CACHE" in name:
        return "CACHE"
    if name in {
        "DELETE_DIRECTORY", "DIRECTORY_SIZE", "FIND_LARGE_FILES", "LIST_DIRECTORY",
        "CREATE_DIRECTORY", "PRINT_WORKING_DIRECTORY", "CHANGE_DIRECTORY",
    }:
        return "DIRECTORY"
    if name in {
        "SHOW_DISK_USAGE", "SHOW_BATTERY", "SHOW_DATE", "WHO_AM_I", "SHOW_HOSTNAME",
        "SHOW_SYSTEM_INFO", "SHOW_UPTIME", "SHOW_MEMORY", "SHOW_CPU", "SHOW_NETWORK",
        "DIAGNOSE_NETWORK",
    }:
        return "SYSTEM"
    if name == "SET_BLUETOOTH_POWER":
        return "CONTROL"
    if name in {"SET_AIRDROP_MODE", "SET_STAGE_MANAGER"}:
        return "CONTROL"
    if name == "FETCH_URL":
        return "WEB"
    if name == "RUN_TOOL":
        return "TOOL"
    return "FILE"


DOMAINS = {
    "APP": "Installed macOS apps: find, open, remove or uninstall applications",
    "FILE": "Individual files: find, open, read, copy, move, rename or delete",
    "DIRECTORY": "Folders and their contents: list, enter, create, delete or inspect size",
    "PROCESS": "Running processes: inspect or terminate a PID by name",
    "PORT": "TCP ports: inspect or stop the process occupying a port number",
    "CACHE": "Named package manager caches: inspect or clean cache",
    "SYSTEM": "This Mac: disk, battery, date, user, hostname, OS, uptime, memory, CPU or network status",
    "CONTROL": "System controls: Bluetooth, AirDrop receiving or Stage Manager",
    "WEB": "HTTP and HTTPS URLs: retrieve a response body with a safe GET request",
    "TOOL": "Run a supported developer tool such as git, brew or cargo",
    "OTHER": "Another request that does not fit these domains",
}

DOMAIN_LABELS = {
    "APP": "applications",
    "FILE": "files",
    "DIRECTORY": "folders",
    "PROCESS": "processes",
    "PORT": "ports",
    "CACHE": "caches",
    "SYSTEM": "computer status",
    "CONTROL": "system controls",
    "WEB": "web requests",
    "TOOL": "developer tools",
    "OTHER": "other",
}

# Laya uses both the label and its description. Natural outcome labels are much
# more reliable than internal enum names such as OPEN_APP or SHOW_MEMORY.
ACTION_LABELS = {
    "FIND_APP": "locate",
    "REMOVE_APP": "uninstall",
    "REMOVE_APP_COMPLETELY": "complete uninstall",
    "OPEN_APP": "launch",
    "QUIT_APP": "close",
    "UPDATE_APP": "update application",
    "FIND_FILE": "locate file",
    "OPEN_FILE": "open file",
    "MOVE_FILE": "move file",
    "RENAME_FILE": "rename file",
    "DELETE_FILE": "trash file",
    "DELETE_DIRECTORY": "trash folder",
    "DIRECTORY_SIZE": "measure folder",
    "FIND_LARGE_FILES": "largest files",
    "FIND_PROCESS": "find process",
    "KILL_PROCESS": "stop process",
    "FIND_PORT_PROCESS": "inspect port",
    "KILL_PORT_PROCESS": "free port",
    "CLEAR_CACHE": "clean cache",
    "SHOW_DISK_USAGE": "storage space",
    "SHOW_BATTERY": "battery level",
    "LIST_PROCESSES": "top processes",
    "SET_BLUETOOTH_POWER": "bluetooth power",
    "SET_AIRDROP_MODE": "AirDrop receiving",
    "SET_STAGE_MANAGER": "Stage Manager",
    "FETCH_URL": "HTTP GET",
    "LIST_DIRECTORY": "list folder",
    "READ_FILE": "read file",
    "COPY_FILE": "copy file",
    "CREATE_DIRECTORY": "make folder",
    "PRINT_WORKING_DIRECTORY": "current folder",
    "CHANGE_DIRECTORY": "enter folder",
    "SHOW_DATE": "date and time",
    "WHO_AM_I": "current user",
    "SHOW_HOSTNAME": "computer name",
    "SHOW_SYSTEM_INFO": "system details",
    "SHOW_UPTIME": "uptime",
    "SHOW_MEMORY": "RAM usage",
    "SHOW_CPU": "CPU details",
    "SHOW_NETWORK": "network addresses",
    "DIAGNOSE_NETWORK": "network diagnosis",
    "RUN_TOOL": "developer tool",
}

ACTION_CRITERIA = {
    "FIND_APP": "locate where the named application is installed",
    "REMOVE_APP": "uninstall or move the named application to Trash",
    "REMOVE_APP_COMPLETELY": "move the named application and its matched user Library data to Trash",
    "OPEN_APP": "make the named application running and visible",
    "QUIT_APP": "stop the named running application but keep it installed",
    "UPDATE_APP": "update the named supported application with its trusted self-updater",
    "FIND_FILE": "find the named file",
    "OPEN_FILE": "open the named file in an application",
    "MOVE_FILE": "move the named file",
    "RENAME_FILE": "rename the named file",
    "DELETE_FILE": "delete the named file",
    "DELETE_DIRECTORY": "move a folder and its contents to Trash",
    "DIRECTORY_SIZE": "report how much storage one folder uses",
    "FIND_LARGE_FILES": "show the largest files inside a folder",
    "FIND_PROCESS": "find one running process by name",
    "KILL_PROCESS": "stop one running process or PID",
    "FIND_PORT_PROCESS": "report which process is listening on a TCP port",
    "KILL_PORT_PROCESS": "stop the process listening on a TCP port",
    "CLEAR_CACHE": "clean a named package manager cache",
    "SHOW_DISK_USAGE": "report free and used storage on the whole disk",
    "SHOW_BATTERY": "report battery charge and power source",
    "LIST_PROCESSES": "show or rank multiple running processes by CPU or memory",
    "SET_BLUETOOTH_POWER": "turn Bluetooth power on or off",
    "SET_AIRDROP_MODE": "turn AirDrop receiving on or off, for contacts or everyone",
    "SET_STAGE_MANAGER": "turn the macOS Stage Manager window layout on or off",
    "FETCH_URL": "make a body-free HTTP or HTTPS GET request and print the response",
    "LIST_DIRECTORY": "show the files and folders inside a folder",
    "READ_FILE": "show what the named text file says",
    "COPY_FILE": "make a copy of the named file",
    "CREATE_DIRECTORY": "create a new folder",
    "PRINT_WORKING_DIRECTORY": "report the terminal's current folder",
    "CHANGE_DIRECTORY": "make the terminal enter another folder",
    "SHOW_DATE": "report the current date or time",
    "WHO_AM_I": "report the current macOS user name",
    "SHOW_HOSTNAME": "report this computer's host name",
    "SHOW_SYSTEM_INFO": "report macOS version, architecture and kernel",
    "SHOW_UPTIME": "report how long since this Mac was started or restarted",
    "SHOW_MEMORY": "report total and currently used RAM or computer memory",
    "SHOW_CPU": "report processor model and core counts",
    "SHOW_NETWORK": "report local network interfaces and IP addresses",
    "DIAGNOSE_NETWORK": "diagnose a slow connection using DNS, Wi-Fi, routing, packet loss, VPN, proxy and port checks",
    "RUN_TOOL": "run a supported git, brew or cargo operation",
}

UNAVAILABLE_GOALS = {
    "SET_DEFAULT_BROWSER": "Change which browser opens web links by default",
    "ENABLE_LOGIN_ITEM": "Make an application launch automatically when logging in",
    "DISABLE_LOGIN_ITEM": "Stop an application from launching automatically when logging in",
    "DOWNLOAD_FILE": "Download a file from the internet",
    "INSTALL_APP": "Install new software or an application",
    "UPDATE_OS": "Update macOS operating system",
    "SEND_EMAIL": "Send an email message to a person",
    "CREATE_ACCOUNT": "Create a new user account",
    "OTHER": "None of these goals describes the request",
}


def unavailable_anchor(text: str, label: str) -> bool:
    lower = text.lower()
    markers = {
        "SET_DEFAULT_BROWSER": ("default browser", "браузером по умолчанию"),
        "ENABLE_LOGIN_ITEM": ("at login", "при входе"),
        "DISABLE_LOGIN_ITEM": ("at login", "при входе"),
        "DOWNLOAD_FILE": ("download ", "скачай", "скачать"),
        "INSTALL_APP": ("install ", "установ", "поставь"),
        "UPDATE_OS": ("update macos", "обнови macos"),
        "SEND_EMAIL": ("send an email", "отправь письмо"),
        "CREATE_ACCOUNT": ("create account", "создай новый аккаунт"),
    }
    return any(marker in lower for marker in markers.get(label, ()))


def strongly_anchored_unavailable(text: str) -> str | None:
    """Recognize explicit unsupported goals without lowering model thresholds."""
    lower = text.lower().strip()
    if "default browser" in lower or "браузером по умолчанию" in lower:
        return "SET_DEFAULT_BROWSER"
    if "at login" in lower or "при входе" in lower:
        if any(marker in lower for marker in (
                "disable", "stop", "don't", "do not", "запрети", "не запуска", "отключ")):
            return "DISABLE_LOGIN_ITEM"
        if any(marker in lower for marker in (
                "enable", "launch", "open", "запуска", "включ")):
            return "ENABLE_LOGIN_ITEM"
    if lower.startswith(("download ", "скачай ", "скачать ")):
        return "DOWNLOAD_FILE"
    if lower.startswith(("install ", "поставь ", "установи ", "установить ")):
        return "INSTALL_APP"
    if "macos" in lower and ("update" in lower or "обнов" in lower):
        return "UPDATE_OS"
    if lower.startswith(("send an email", "send email", "отправь письмо")):
        return "SEND_EMAIL"
    if lower.startswith(("create account", "create an account", "создай новый аккаунт")):
        return "CREATE_ACCOUNT"
    return None


def understand_unavailable(text: str, agent, threshold: float) -> tuple[str, float] | None:
    anchored = strongly_anchored_unavailable(text)
    if anchored is not None:
        return anchored, 1.0
    lower = text.lower()
    if ("at login" in lower or "при входе" in lower or
            ("macos" in lower and ("update" in lower or "обнов" in lower))):
        criteria = {
            "ENABLE_LOGIN_ITEM": "Enable an app to automatically open at login",
            "DISABLE_LOGIN_ITEM": "Prevent an app from automatically opening at login",
            "INSTALL_APP": "Install an application on this Mac",
            "UPDATE_OS": "Install macOS operating system updates",
            "OTHER": "A different goal, not one of those above",
        }
    else:
        criteria = UNAVAILABLE_GOALS
    raw = agent.predict(text, {"goal": {
        "type": "choice",
        "instructions": ("Choose the exact goal or OTHER." if criteria is not UNAVAILABLE_GOALS
                         else "Infer the user's actual goal. Choose OTHER if it does not fit a listed goal."),
        "criteria": criteria,
    }})
    selected = choice(raw, "goal", set(criteria))
    if selected is None or selected[0] == "OTHER" or selected[1] < max(threshold, 0.85):
        return None
    return selected if unavailable_anchor(text, selected[0]) else None


def questions(capabilities: list[dict], selected: str | None = None) -> dict:
    if selected is None:
        domains = {domain(item["name"]) for item in capabilities}
        return {"domain": {
            "type": "choice",
            "instructions": "What should the computer help the user with? Choose other if none fit.",
            "criteria": {DOMAIN_LABELS[name]: description for name, description in DOMAINS.items()
                         if name in domains or name == "OTHER"},
        }}
    relevant = [item for item in capabilities if domain(item["name"]) == selected]
    criteria = {ACTION_LABELS[item["name"]]: ACTION_CRITERIA.get(item["name"], item["description"])
                for item in relevant}
    criteria["none"] = "do none of the listed operations"
    return {"action": {
        "type": "choice",
        "instructions": "What should the computer do for the user? Choose none if no listed operation fits.",
        "criteria": criteria,
    }}


def choice(raw: dict, question: str, allowed: set[str]) -> tuple[str, float] | None:
    answer = raw.get("answers", {}).get(question, {})
    label = answer.get("choice")
    probability = answer.get("probabilities", {}).get(label)
    if label not in allowed or isinstance(probability, bool) or not isinstance(probability, (int, float)) or not 0 <= probability <= 1:
        return None
    return label, probability


def classify(payload: dict, agent=None) -> dict:
    text = payload.get("text")
    capabilities = payload.get("capabilities")
    if not isinstance(text, str) or not text.strip() or len(text) > 4096 or not isinstance(capabilities, list):
        return {"action": "UNCLEAR", "confidence": 0}
    threshold = payload.get("threshold", 0.70)
    if isinstance(threshold, bool) or not isinstance(threshold, (int, float)) or not 0.5 <= threshold <= 1:
        return {"action": "UNCLEAR", "confidence": 0}
    if agent is None:
        agent = get_agent()
    present_domains = {domain(item["name"]) for item in capabilities}
    diagnostics = {}
    if len(present_domains) == 1:
        domain_name, domain_confidence = next(iter(present_domains)), 1.0
    else:
        domain_raw = agent.predict(text, questions(capabilities))
        label_to_domain = {DOMAIN_LABELS[name]: name for name in present_domains | {"OTHER"}}
        selected = choice(domain_raw, "domain", set(label_to_domain))
        if selected is None:
            return {"action": "UNCLEAR", "confidence": 0}
        domain_name, domain_confidence = label_to_domain[selected[0]], selected[1]
        diagnostics["domain"] = domain_raw.get("answers", {}).get("domain", {}).get("probabilities", {})
    if domain_name == "OTHER" and domain_confidence >= threshold:
        result = {"action": "UNSUPPORTED", "confidence": domain_confidence}
    elif domain_confidence < threshold:
        result = {"action": "UNCLEAR", "confidence": domain_confidence}
    else:
        action_raw = agent.predict(text, questions(capabilities, domain_name))
        label_to_action = {ACTION_LABELS[item["name"]]: item["name"] for item in capabilities
                           if domain(item["name"]) == domain_name}
        selected_action = choice(action_raw, "action", set(label_to_action) | {"none"})
        diagnostics["action"] = action_raw.get("answers", {}).get("action", {}).get("probabilities", {})
        if selected_action is None:
            result = {"action": "UNCLEAR", "confidence": 0}
        elif selected_action[0] == "none":
            result = {"action": "UNSUPPORTED", "confidence": selected_action[1]}
        elif selected_action[1] >= threshold:
            result = {"action": label_to_action[selected_action[0]], "confidence": selected_action[1]}
        else:
            result = {"action": "UNCLEAR", "confidence": selected_action[1]}
    if payload.get("debug") is True:
        result["diagnostics"] = diagnostics
    if result["action"] in ("UNSUPPORTED", "UNCLEAR"):
        understood = understand_unavailable(text, agent, threshold)
        if understood is not None:
            result["action"] = "UNSUPPORTED"
            result["semantic_goal"] = understood[0]
            result["semantic_confidence"] = understood[1]
    return result


def get_agent():
    global _agent
    if _agent is None:
        import mlx.core as mx
        import laya_mlx as laya
        mx.set_cache_limit(512 * 1024 * 1024)
        _agent = laya.load(MODEL, dtype="float16", batch_size=16, cache_prompts=True)
    return _agent


def serve(socket_path: str) -> None:
    path = Path(socket_path)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(path.parent, 0o700)
    lock_path = path.parent / "daemon.lock"
    with open(lock_path, "a+b") as lock:
        os.chmod(lock_path, 0o600)
        try:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return
        if os.path.lexists(path):
            try:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as existing:
                    existing.settimeout(0.2)
                    existing.connect(str(path))
                return
            except OSError:
                path.unlink()
        own_inode = None
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
                server.bind(str(path))
                own_inode = path.stat().st_ino
                os.chmod(path, 0o600)
                server.listen(8)
                deadline = time.monotonic() + TIMEOUT
                while True:
                    server.settimeout(max(0.1, deadline - time.monotonic()))
                    try:
                        connection, _ = server.accept()
                    except socket.timeout:
                        break
                    with connection:
                        connection.settimeout(180)
                        with connection.makefile("rwb") as stream:
                            try:
                                raw = stream.readline(65537)
                                if not raw:
                                    continue
                                if len(raw) > 65536:
                                    raise ValueError("request too large")
                                payload = json.loads(raw)
                                if payload.get("control") == "status":
                                    result = {"ok": True, "loaded": _agent is not None, "pid": os.getpid(), "idle_timeout": TIMEOUT, "model": MODEL}
                                    if _agent is not None:
                                        import mlx.core as mx
                                        result["mlx_active_bytes"] = int(mx.get_active_memory())
                                        result["mlx_cache_bytes"] = int(mx.get_cache_memory())
                                        result["mlx_peak_bytes"] = int(mx.get_peak_memory())
                                elif payload.get("control") == "stop":
                                    result = {"ok": True, "stopping": True}
                                else:
                                    result = classify(payload)
                                    deadline = time.monotonic() + TIMEOUT
                                stream.write((json.dumps(result) + "\n").encode())
                                stream.flush()
                                if payload.get("control") == "stop":
                                    break
                            except Exception as exc:
                                try:
                                    stream.write((json.dumps({"error": f"{type(exc).__name__}: {exc}"}) + "\n").encode())
                                    stream.flush()
                                except OSError:
                                    pass
        finally:
            if own_inode is not None and os.path.lexists(path) and path.lstat().st_ino == own_inode:
                path.unlink()


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: server.py <socket_path>")
    serve(sys.argv[1])
