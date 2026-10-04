use crate::intent::{Action, Intent, SemanticGoal};
use crate::lexicon::{self, Verb};
use crate::system;

pub const MAX_INPUT_BYTES: usize = 4_096;

/// A fast-path decision: a typed intent, a goal TerFinder understands but does
/// not execute, or an explicit refusal with a reason.
#[derive(Debug)]
pub enum Parsed {
    Intent(Intent),
    Unsupported(SemanticGoal),
    Refused(String),
}

fn words(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|s| {
            s.trim_matches(|c: char| ",.!?;:".contains(c))
                .to_lowercase()
        })
        .collect()
}

fn rest(original: &str, n: usize) -> String {
    original
        .split_whitespace()
        .skip(n)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn port(words: &[String]) -> Option<u16> {
    words
        .iter()
        .find_map(|w| w.parse::<u16>().ok().filter(|n| *n > 0))
}

fn mentions_bluetooth(words: &[String], lower: &str) -> bool {
    lower.contains("blue tooth")
        || words.iter().any(|word| {
            matches!(
                word.as_str(),
                "bluetooth" | "блютуз" | "блютус" | "блутуз" | "блутус"
            ) || (word.chars().count() >= 6
                && (lexicon::edit1(word, "bluetooth") || lexicon::edit1(word, "блютуз")))
        })
}

fn mentions_airdrop(lower: &str) -> bool {
    ["airdrop", "air drop", "эйрдроп", "эирдроп", "аирдроп"]
        .iter()
        .any(|name| lower.contains(name))
}

fn mentions_stage_manager(lower: &str) -> bool {
    [
        "stage manager",
        "stagemanager",
        "stage-manager",
        "стейдж менеджер",
        "стейджменеджер",
        "постановщик",
    ]
    .iter()
    .any(|name| lower.contains(name))
}

fn toggle_word(lower: &str, expected: &str) -> bool {
    lower
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .any(|word| word == expected)
        && ["turn ", "switch ", "set "]
            .iter()
            .any(|verb| lower.starts_with(verb))
}

fn requested_toggle(first: Option<Verb>, first_raw: &str, lower: &str) -> Option<&'static str> {
    if first == Some(Verb::Kill)
        || lower.contains("turn off")
        || toggle_word(lower, "off")
        || lower.contains("disable")
        || lower.contains("отключ")
        || lower.contains("выключ")
        || matches!(first_raw, "off")
    {
        Some("off")
    } else if first == Some(Verb::Open)
        || lower.contains("turn on")
        || toggle_word(lower, "on")
        || lower.contains("enable")
        || lower.contains("активир")
        || lower.contains("включ")
        || matches!(first_raw, "on")
    {
        Some("on")
    } else {
        None
    }
}

fn http_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find_map(lexicon::normalize_http_target)
}

fn http_get_request(text: &str) -> Result<Option<Intent>, String> {
    let lower = text.to_lowercase();
    let explicit = [
        "curl ",
        "fetch ",
        "request ",
        "get ",
        "http get ",
        "дерни ",
        "дёрни ",
        "call ",
    ]
    .iter()
    .any(|marker| lower.starts_with(marker));
    let request_noun = [
        "curl",
        "request",
        "запрос",
        "запит",
        "requête",
        "requete",
        "anfrage",
        "solicitud",
        "requisição",
        "requisicao",
        "richiesta",
        "żądanie",
        "zadanie",
        "istek",
        "verzoek",
        "požadavek",
        "pozadavek",
        "请求",
        "リクエスト",
        "요청",
        "طلب",
        "अनुरोध",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let action_verb = [
        "отправ",
        "сдел",
        "выполн",
        "пошли",
        "дёрн",
        "дерн",
        "получ",
        "надіш",
        "зроб",
        "send",
        "make",
        "perform",
        "issue",
        "fetch",
        "call",
        "envía",
        "envia",
        "haz",
        "realiza",
        "sende",
        "mach",
        "führe",
        "fuhre",
        "envoie",
        "fais",
        "effectue",
        "envie",
        "faça",
        "faca",
        "invia",
        "fai",
        "wyślij",
        "wyslij",
        "zrób",
        "zrob",
        "gönder",
        "gonder",
        "yap",
        "stuur",
        "doe",
        "pošli",
        "posli",
        "proveď",
        "proved",
        "发送",
        "送信",
        "보내",
        "أرسل",
        "ارسل",
        "भेज",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let mutating_method = lower
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .any(|word| matches!(word, "post" | "put" | "patch" | "delete"));
    let requested = explicit || (request_noun && (action_verb || mutating_method));
    if !requested {
        return Ok(None);
    }
    if ["\n", "\r", "\0", "`", "$(", "; ", " | ", " > ", " < ", "& "]
        .iter()
        .any(|marker| text.contains(marker))
    {
        return Err("Shell syntax is not accepted in HTTP requests. Nothing was sent.".into());
    }
    let Some(url) = http_url(text) else {
        return Ok(None);
    };
    let words = lower
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>();
    if words
        .iter()
        .any(|word| matches!(*word, "post" | "put" | "patch" | "delete"))
    {
        return Err("Only body-free HTTP GET requests are supported. Nothing was sent.".into());
    }
    if lower.split_whitespace().any(|word| word.starts_with('-')) {
        return Err(
            "Curl flags are not accepted; use a plain HTTP(S) URL. Nothing was sent.".into(),
        );
    }
    Ok(Some(Intent::target(Action::FetchUrl, url)))
}

fn pathish_object(text: &str) -> Option<String> {
    text.split_whitespace().rev().find_map(|raw| {
        let value = raw.trim_matches(|c: char| "!?;,:([]){}`\"'".contains(c));
        (!value.is_empty()
            && (value.contains(['/', '~', '.']) || value.eq_ignore_ascii_case("hosts")))
        .then(|| value.to_string())
    })
}

fn explicitly_negated_action(flat: &str) -> bool {
    [
        "do not open",
        "don't open",
        "dont open",
        "don t open",
        "do not delete",
        "don't delete",
        "dont delete",
        "don t delete",
        "do not remove",
        "don't remove",
        "don t remove",
        "do not close",
        "don't close",
        "don t close",
        "do not stop",
        "don't stop",
        "don t stop",
        "do not move",
        "don't move",
        "don t move",
        "do not request",
        "don't request",
        "don t request",
        "do not make request",
        "don t make request",
        "do not fetch",
        "don't fetch",
        "don t fetch",
        "do not use curl",
        "don't use curl",
        "don t use curl",
        "do not enable",
        "don t enable",
        "do not disable",
        "don t disable",
        "do not turn",
        "don t turn",
        "do not switch",
        "don t switch",
        "не открывай",
        "не удаляй",
        "не закрывай",
        "не останавливай",
        "не перемещай",
        "не включай",
        "не выключай",
        "не активируй",
        "не отключай",
        "не делай запрос",
        "не отправляй запрос",
        "не выполняй curl",
        "не используй curl",
        "no abras",
        "no borres",
        "nicht öffnen",
        "nicht offnen",
        "nicht löschen",
        "nicht loschen",
        "ne ouvre pas",
        "ne supprime pas",
        "non aprire",
        "non eliminare",
    ]
    .iter()
    .any(|phrase| flat.contains(phrase))
}

/// High-signal conversational requests that do not depend on command-shaped
/// wording. These rules only select registered typed actions and extract
/// concrete objects already present in the request.
fn conversational(text: &str, flat: &str) -> Option<Intent> {
    let lower = text.to_lowercase();
    let contains_any = |markers: &[&str]| markers.iter().any(|marker| lower.contains(marker));

    if contains_any(&[
        "what folder this terminal is in",
        "what directory this terminal is in",
        "what folder this shell is in",
        "which folder am i in",
        "напомни в какой папке",
        "где сейчас терминал",
    ]) {
        return Some(Intent::new(Action::PrintWorkingDirectory));
    }
    if contains_any(&[
        "account is this shell using",
        "account am i using",
        "какой аккаунт сейчас используется",
    ]) {
        return Some(Intent::new(Action::WhoAmI));
    }
    if contains_any(&[
        "kind of mac",
        "about this mac",
        "что у меня за мак",
        "что это за mac",
    ]) {
        return Some(Intent::new(Action::ShowSystemInfo));
    }
    if (contains_any(&["chip", "cores", "ядра", "чип"])
        && contains_any(&["machine", "computer", "mac", "компьютер", "мак"]))
        || contains_any(&["what cpu", "какой cpu"])
    {
        return Some(Intent::new(Action::ShowCpu));
    }
    if contains_any(&["local network", "локальн", "wi-fi", "wifi"])
        && contains_any(&["address", "адрес", "ip"])
    {
        return Some(Intent::new(Action::ShowNetwork));
    }
    if contains_any(&["tell me when it is", "what day is it", "какой сейчас день"])
        || (contains_any(&["time", "время"]) && contains_any(&["now", "current", "сейчас"]))
    {
        return Some(Intent::new(Action::ShowDate));
    }

    if mentions_bluetooth(&words(text), &lower) {
        if contains_any(&[
            "cut",
            "shut",
            "disable",
            "kill",
            "без bluetooth",
            "отруби",
            "погаси",
        ]) {
            return Some(Intent::target(Action::SetBluetoothPower, "off"));
        }
        if contains_any(&["restore", "enable", "bring back", "верни", "подними"]) {
            return Some(Intent::target(Action::SetBluetoothPower, "on"));
        }
    }

    if let Some(target) = pathish_object(text) {
        if contains_any(&[
            "where did",
            "where has",
            "end up",
            "где лежит",
            "куда делся",
        ]) {
            return Some(Intent::target(Action::FindFile, target));
        }
        if contains_any(&[
            "bring",
            "usual app",
            "default app",
            "покажи в приложении",
            "отобрази в приложении",
        ]) {
            return Some(Intent::target(Action::OpenFile, target));
        }
        if contains_any(&[
            "do not need",
            "don't need",
            "dont need",
            "no longer need",
            "больше не нужен",
            "больше не нужна",
        ]) {
            return Some(Intent::target(Action::DeleteFile, target));
        }
    }

    if let Some(directory) = lexicon::canonical_dir(flat) {
        if contains_any(&[
            "storage hog",
            "space hog",
            "что съело место",
            "что жрёт место",
        ]) {
            let mut intent = Intent::target(Action::FindLargeFiles, directory);
            intent.minimum_size_bytes = Some(0);
            return Some(intent);
        }
        if contains_any(&[
            "take this shell",
            "take the shell",
            "switch this shell",
            "перенеси терминал",
            "перейди терминалом",
        ]) {
            return Some(Intent::target(Action::ChangeDirectory, directory));
        }
    }

    if lower.contains("process") || lower.contains("процесс") {
        let tokens = words(text);
        if contains_any(&[" alive", "running", "запущен", "жив ли", "работает ли"])
            && let Some(index) = tokens.iter().position(|token| {
                matches!(
                    token.as_str(),
                    "process"
                        | "процесс"
                        | "процес"
                        | "prozess"
                        | "proceso"
                        | "processo"
                        | "süreç"
                        | "surec"
                        | "进程"
                        | "プロセス"
                        | "프로세스"
                        | "عملية"
                        | "प्रक्रिया"
                )
            })
            && index > 0
        {
            let target = tokens[index - 1].clone();
            if !["a", "the", "этот", "какой", "one"].contains(&target.as_str()) {
                return Some(Intent::target(Action::FindProcess, target));
            }
        }
    }

    if let Some(app) = system::mentioned_app(flat) {
        if contains_any(&[
            "feel like using",
            "сейчас нужен",
            "сейчас нужна",
            "voudrais utiliser",
        ]) {
            return Some(Intent::target(Action::OpenApp, app));
        }
        if lower.contains("installed")
            && contains_any(&[
                "do not want",
                "don't want",
                "dont want",
                "anymore",
                "no longer",
            ])
        {
            return Some(Intent::target(Action::RemoveApp, app));
        }
        if contains_any(&[
            "done with",
            "finished with",
            "больше не пользуюсь",
            "пока не нужен",
        ]) {
            return Some(Intent::target(Action::QuitApp, app));
        }
    }
    None
}

fn process_ranking(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    let has_any = |markers: &[&str]| markers.iter().any(|marker| lower.contains(marker));
    let tokens = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let memory = has_any(&[
        "памят",
        "оператив",
        "ram",
        "memory",
        "memoria",
        "mémoire",
        "memoire",
        "speicher",
        "memória",
        "pamię",
        "pamie",
        "bellek",
        "geheugen",
        "pamě",
        "内存",
        "メモリ",
        "메모리",
        "الذاكرة",
        "मेमोरी",
    ]);
    let cpu = has_any(&[
        " cpu",
        "cpu ",
        "процессор",
        "processor",
        "prozessor",
        "procesador",
        "processeur",
        "processore",
        "işlemci",
        "islemci",
        "处理器",
        "プロセッサ",
        "프로세서",
        "المعالج",
        "प्रोसेसर",
    ]);
    let process = tokens.iter().any(|token| {
        [
            "процесс",
            "процесса",
            "процессы",
            "процессов",
            "process",
            "processes",
            "prozess",
            "prozesse",
            "proceso",
            "procesos",
            "processus",
            "processo",
            "processi",
            "proces",
            "procesy",
            "süreç",
            "surec",
            "进程",
            "プロセス",
            "프로세스",
            "عملية",
            "عمليات",
            "प्रक्रिया",
            "प्रक्रियाएं",
        ]
        .contains(token)
    });
    let consumer = has_any(&[
        "жрет",
        "жрёт",
        "ест ",
        "занимает",
        "потребляет",
        "использует",
        "грузит",
        "using",
        "uses",
        "consum",
        "takes",
        "hog",
        "top ",
        "больше всего",
        "самые",
        "most",
        "highest",
        "máximo",
        "maximo",
        "plus",
        "meiste",
        "最多",
    ]);
    if memory && (process || consumer) {
        Some("memory")
    } else if cpu && (process || consumer) {
        Some("cpu")
    } else {
        None
    }
}

const DISK_PHRASES: &[&str] = &[
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
];

const SIZE_OPENERS: &[&str] = &[
    "весит",
    "вес",
    "размер",
    "size",
    "du",
    "места",
    "место",
    "сколько",
    "стоит",
    "занима",
];

const BIG_MARKERS: &[&str] = &[
    "больш",
    "large",
    "big",
    "крупн",
    "жирн",
    "heavy",
    "тяжел",
    "самы",
    "огромн",
    "largest",
];

const FILE_MARKERS: &[&str] = &[
    "файл",
    "file",
    "видео",
    "video",
    "pdf",
    "фото",
    "image",
    "picture",
    "картин",
    "ролик",
    "клип",
    "movie",
    "документ",
];

const RELOCATION_SEPARATORS: &[&str] = &[
    "to",
    "into",
    "as",
    "in",
    "at",
    "на",
    "в",
    "до",
    "як",
    "как",
    "a",
    "para",
    "hacia",
    "como",
    "nach",
    "zu",
    "als",
    "vers",
    "dans",
    "en",
    "pour",
    "em",
    "come",
    "do",
    "jako",
    "naar",
    "olarak",
    "içine",
    "إلى",
    "الى",
    "में",
    "到",
    "に",
    "에",
];

const CD_FILLERS: &[&str] = &[
    "to",
    "in",
    "into",
    "at",
    "в",
    "на",
    "до",
    "a",
    "the",
    "папку",
    "папке",
    "folder",
    "dir",
    "carpeta",
    "ordner",
    "dossier",
    "pasta",
    "cartella",
    "folderu",
    "složky",
    "klasör",
    "map",
    "إلى",
    "الى",
    "में",
];

const SYSTEM_INFO_PHRASES: &[&str] = &[
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
];

const UPTIME_PHRASES: &[&str] = &[
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
];

const MEMORY_PHRASES: &[&str] = &[
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
];

const CPU_PHRASES: &[&str] = &[
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
];

const NETWORK_PHRASES: &[&str] = &[
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
];

/// Drop leading size verbs and openers so the object phrase is left intact
/// (`сколько весит anton` → `anton`).
fn strip_leading_openers(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        let Some(first) = trimmed.split_whitespace().next() else {
            break;
        };
        if first.contains(['/', '~']) {
            break;
        }
        let lower = first
            .trim_matches(|c: char| "!?,.;:\"'".contains(c))
            .to_lowercase();
        let opener = matches!(lexicon::verb(&lower), Some(Verb::Size) | Some(Verb::Find))
            || SIZE_OPENERS.iter().any(|word| {
                if word.len() <= 3 {
                    lower == *word
                } else {
                    lower.contains(word)
                }
            });
        if !opener {
            break;
        }
        text = &trimmed[first.len()..];
    }
    text.trim()
}

/// Resolve a size phrasing's object into an existing directory. A full path
/// keeps every component; otherwise well-known home directories keep their
/// canonical name and free-form names resolve against the current directory,
/// home, and the home folder name itself.
fn size_target(object: &str) -> Option<String> {
    let object = lexicon::strip_object_filler(object);
    let object = object
        .trim()
        .trim_start_matches(|c: char| "\"'".contains(c))
        .trim_end_matches(|c: char| "?!,.;:\"'".contains(c));
    if object.is_empty() {
        return None;
    }
    if object.contains('/') || object.starts_with('~') {
        return system::existing_dir(object).map(|path| path.display().to_string());
    }
    if let Some(dir) = lexicon::canonical_dir(object) {
        return Some(dir.to_string());
    }
    system::existing_dir(object).map(|path| path.display().to_string())
}

fn complete_app_removal(text: &str) -> Option<Intent> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    let prefixes = [
        "полностью удали ",
        "полностью удалить ",
        "удали полностью ",
        "удалить полностью ",
        "completely uninstall ",
        "completely remove ",
        "fully uninstall ",
        "fully remove ",
        "uninstall completely ",
        "remove completely ",
    ];
    let prefix = prefixes
        .into_iter()
        .find(|prefix| lower.starts_with(prefix))?;
    let raw_target = &trimmed[prefix.len()..];
    let target = lexicon::strip_app_prefix(&lexicon::strip_object_filler(raw_target));
    let safe_name = !target.is_empty()
        && !lexicon::is_vague_object(&target)
        && !target.contains(['/', '~'])
        && target.split_whitespace().count() <= 8
        && target.chars().all(|character| {
            character.is_alphanumeric()
                || character.is_whitespace()
                || matches!(character, '-' | '_' | '.' | '+' | '&' | '\'' | '(' | ')')
        });
    safe_name.then(|| Intent::target(Action::RemoveAppCompletely, target))
}

fn slow_internet_request(lower: &str) -> bool {
    let internet = lower.contains("интернет")
        || lower.contains("internet")
        || lower.contains("network")
        || lower.contains("wi-fi")
        || lower.contains("wifi");
    let diagnostic = lower.contains("тормозит")
        || lower.contains("медлен")
        || lower.contains("slow")
        || lower.contains("почему")
        || lower.contains("diagnos")
        || lower.contains("диагност")
        || lower.contains("проблем");
    internet && diagnostic
}

/// Strict deterministic rules. `Ok(None)` means the request is outside the
/// known phrasings so the caller may fall back to the model. `Err` is an
/// explicit refusal with a reason.
fn strict(text: &str, flat: &str) -> Result<Option<Intent>, String> {
    let w = words(text);
    if w.is_empty() {
        return Ok(None);
    }
    if lexicon::unsupported_goal(flat).is_some() {
        return Ok(None);
    }
    let first_raw = w[0].as_str();
    let first = lexicon::verb(first_raw);
    let has = |items: &[&str]| items.iter().any(|x| w.iter().any(|y| y == x));
    let lower = text.to_lowercase();

    if let Some(intent) = complete_app_removal(text) {
        return Ok(Some(intent));
    }
    if slow_internet_request(&lower) {
        if [
            "не провер",
            "не диагност",
            "do not diagnose",
            "don't diagnose",
            "dont diagnose",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            return Err(
                "The network diagnosis is explicitly negated. Nothing was executed.".into(),
            );
        }
        return Ok(Some(Intent::new(Action::DiagnoseNetwork)));
    }

    if let Some(sort) = process_ranking(&lower) {
        return Ok(Some(Intent::target(Action::ListProcesses, sort)));
    }

    if mentions_airdrop(&lower) {
        let mode = if [
            "только контакт",
            "только для контакт",
            "contacts only",
            "contact only",
            "only for contacts",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            Some("contacts")
        } else if ["для всех", "everyone", "everybody", "all people"]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            Some("everyone")
        } else {
            requested_toggle(first, first_raw, &lower)
        };
        if let Some(mode) = mode {
            return Ok(Some(Intent::target(Action::SetAirDropMode, mode)));
        }
    }

    if mentions_stage_manager(&lower)
        && let Some(power) = requested_toggle(first, first_raw, &lower)
    {
        return Ok(Some(Intent::target(Action::SetStageManager, power)));
    }

    if mentions_bluetooth(&w, &lower)
        && let Some(power) = requested_toggle(first, first_raw, &lower)
    {
        return Ok(Some(Intent::target(Action::SetBluetoothPower, power)));
    }

    if ((first == Some(Verb::Size)
        || first == Some(Verb::Find)
        || first_raw == "battery"
        || first_raw == "заряд")
        && (lower.contains("зарядк")
            || lower.contains("заряд батар")
            || lower.contains("battery level")
            || lower.contains("battery charge")
            || lower.contains("battery percentage")))
        || lower.contains("البطاري")
    {
        return Ok(Some(Intent::new(Action::ShowBattery)));
    }
    let plural_processes = has(&[
        "процессы",
        "процесів",
        "процеси",
        "processes",
        "prozesse",
        "procesos",
        "processus",
        "processos",
        "processi",
        "procesy",
        "processen",
        "procesy",
        "进程",
        "プロセス",
        "프로세스",
        "عمليات",
        "प्रक्रियाएं",
    ]);
    let plural_processes_at_end = w.last().is_some_and(|token| {
        [
            "процессы",
            "процесів",
            "процеси",
            "processes",
            "prozesse",
            "procesos",
            "processus",
            "processos",
            "processi",
            "procesy",
            "processen",
            "进程",
            "プロセス",
            "프로세스",
            "عمليات",
            "प्रक्रियाएं",
        ]
        .contains(&token.as_str())
    });
    let asks_for_process_list = (first == Some(Verb::Find)
        || lower.starts_with("list ")
        || lower.starts_with("какие ")
        || lower.starts_with("какой ")
        || lower.starts_with("which ")
        || lower.starts_with("what "))
        && ((plural_processes && plural_processes_at_end)
            || lower.contains("работающ")
            || lower.contains("запущен")
            || lower.contains("running")
            || lower.contains("жрут")
            || lower.contains("больше всего")
            || lower.contains("самые")
            || lower.contains("топ")
            || lower.contains("top ")
            || lower.ends_with("top")
            || lower.contains("most "))
        && (lower.contains("процесс") || lower.contains("process"));
    if asks_for_process_list {
        return Ok(Some(Intent::new(Action::ListProcesses)));
    }

    if DISK_PHRASES.iter().any(|phrase| flat.contains(phrase)) {
        return Ok(Some(Intent::new(Action::ShowDiskUsage)));
    }

    if is_pwd_request(first_raw, &w, flat) {
        return Ok(Some(Intent::new(Action::PrintWorkingDirectory)));
    }
    if let Some(dir) = listing_phrase_dir(lower.as_str(), text) {
        return Ok(Some(if dir.is_empty() {
            Intent::new(Action::ListDirectory)
        } else {
            Intent::target(Action::ListDirectory, dir)
        }));
    }
    if matches!(flat, "кто я" | "who am i" | "whoami") {
        return Ok(Some(Intent::new(Action::WhoAmI)));
    }
    if matches!(
        flat,
        "date"
            | "какое сегодня число"
            | "который час"
            | "what time is it"
            | "what's the time"
            | "whats the time"
            | "hora actual"
    ) {
        return Ok(Some(Intent::new(Action::ShowDate)));
    }
    if matches!(
        flat,
        "hostname" | "как называется компьютер" | "computer name" | "nombre del equipo"
    ) {
        return Ok(Some(Intent::new(Action::ShowHostname)));
    }
    if SYSTEM_INFO_PHRASES.contains(&flat) {
        return Ok(Some(Intent::new(Action::ShowSystemInfo)));
    }
    if UPTIME_PHRASES.contains(&flat) {
        return Ok(Some(Intent::new(Action::ShowUptime)));
    }
    if MEMORY_PHRASES.contains(&flat) {
        return Ok(Some(Intent::new(Action::ShowMemory)));
    }
    if (lower.contains("оператив") || lower.contains("ram"))
        && [
            "занят",
            "заполн",
            "использ",
            "свобод",
            "остал",
            "usage",
            "used",
            "available",
            "free",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Ok(Some(Intent::new(Action::ShowMemory)));
    }
    if CPU_PHRASES.contains(&flat) {
        return Ok(Some(Intent::new(Action::ShowCpu)));
    }
    if NETWORK_PHRASES.contains(&flat) {
        return Ok(Some(Intent::new(Action::ShowNetwork)));
    }

    if [
        "что написано",
        "что там написано",
        "что говорится",
        "содержимое файла",
        "what it says",
        "what is written",
        "file contents",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        && let Some(target) = pathish_object(text)
    {
        return Ok(Some(Intent::target(Action::ReadFile, target)));
    }

    for prefix in [
        "find содержимое файла ",
        "find содержимое ",
        "find contents of ",
        "find content of ",
        "list содержимое файла ",
        "list содержимое ",
        "list contents of ",
    ] {
        if lower.strip_prefix(prefix).is_some() {
            let target = rest(text, prefix.split_whitespace().count());
            if !target.is_empty() {
                return Ok(Some(Intent::target(Action::ReadFile, target)));
            }
        }
    }

    if let Some(target) = lower.strip_prefix("how large is ")
        && ["downloads", "documents", "desktop"].contains(&target.trim())
    {
        return Ok(Some(Intent::target(Action::DirectorySize, rest(text, 3))));
    }
    if lower.starts_with("show largest files in ") {
        let target = lexicon::strip_object_filler(rest(text, 4).as_str());
        if ["downloads", "documents", "desktop"].contains(&target.as_str()) {
            let mut intent = Intent::target(Action::FindLargeFiles, target);
            intent.minimum_size_bytes = Some(0);
            intent.file_type = lexicon::file_type(flat);
            intent.max_age_days = lexicon::age_days(flat)?;
            return Ok(Some(intent));
        }
    }
    if lower.starts_with("покажи размер папки ") {
        return Ok(Some(Intent::target(Action::DirectorySize, rest(text, 3))));
    }

    let number = w.iter().find_map(|token| token.parse::<u32>().ok());
    let port_words = [
        "порт",
        "порту",
        "port",
        "на",
        "on",
        "listening",
        "слушает",
        "сидит",
        "занимает",
        "держит",
    ];
    if let Some(pid) = number
        && (has(&["pid", "пид"])
            || lower.contains("что за процесс")
            || lower.contains("what process has"))
        && !has(&["порт", "порту", "port"])
        && first != Some(Verb::Kill)
        && (has(&["кто", "что", "какой", "who", "what", "which"])
            || lower.contains("что за")
            || lower.contains("who is")
            || first == Some(Verb::Find))
    {
        let mut intent = Intent::new(Action::FindProcess);
        intent.pid = Some(pid);
        return Ok(Some(intent));
    }
    if first == Some(Verb::Kill)
        && let Some(pid) = number
        && !has(&port_words)
        && (has(&["pid", "пид", "процесс", "process"])
            || ["kill", "убей", "убить", "terminate", "заверши"].contains(&first_raw))
    {
        let mut intent = Intent::new(Action::KillProcess);
        intent.pid = Some(pid);
        return Ok(Some(intent));
    }

    if let Some(port) = port(&w) {
        let kill = first == Some(Verb::Kill);
        let names_port = has(&["порт", "порту", "port"]);
        let asks_about_owner = has(&[
            "кто",
            "что",
            "чей",
            "какой",
            "who",
            "what",
            "whose",
            "uses",
            "using",
            "grabbed",
            "listening",
            "слушает",
            "сидит",
            "занимает",
            "держит",
            "occupied",
            "free",
            "освободи",
        ]);
        let port_context = (names_port
            && (first == Some(Verb::Find)
                || kill
                || matches!(first_raw, "port" | "порт")
                || asks_about_owner))
            || ((first == Some(Verb::Find) || kill || has(&["процесс", "process"]))
                && asks_about_owner)
            || (has(&["кто", "что", "какой", "who", "what", "which"])
                && has(&[
                    "listening",
                    "слушает",
                    "сидит",
                    "занимает",
                    "держит",
                    "occupied",
                    "grabbed",
                    "uses",
                ]))
            || (kill && has(&["процесс", "process"]))
            || (w.len() <= 4 && has(&["кто", "что", "who", "whos", "what's"]));
        if port_context {
            let mut intent = Intent::new(if kill {
                Action::KillPortProcess
            } else {
                Action::FindPortProcess
            });
            intent.port = Some(port);
            return Ok(Some(intent));
        }
    }
    if first_raw == "port" || first_raw == "порт" {
        return Ok(None);
    }

    if (first == Some(Verb::Find) || lower.starts_with("where is "))
        && has(&["файлы", "files"])
        && has(&["больше", "over", "larger", "than", "более"])
        && let Some(count) = w.iter().find_map(|token| token.parse::<u64>().ok())
    {
        let multiplier = if has(&["гб", "gb", "gib"]) {
            1_073_741_824
        } else if has(&["мб", "mb", "mib"]) {
            1_048_576
        } else {
            0
        };
        let location = w.iter().position(|token| token == "в" || token == "in");
        if multiplier > 0
            && let Some(location) = location
        {
            let mut intent = Intent::target(Action::FindLargeFiles, rest(text, location + 1));
            intent.minimum_size_bytes = count.checked_mul(multiplier);
            if intent.minimum_size_bytes.is_some() {
                intent.file_type = lexicon::file_type(flat);
                intent.max_age_days = lexicon::age_days(flat)?;
                return Ok(Some(intent));
            }
        }
    }

    if let Some(dir) = lexicon::canonical_dir(flat) {
        let big = BIG_MARKERS.iter().any(|marker| flat.contains(marker));
        let files = FILE_MARKERS.iter().any(|marker| flat.contains(marker));
        let size = lexicon::has_size_marker(flat);
        let find = first == Some(Verb::Find);
        let threshold = lexicon::min_size(flat);
        if threshold.is_some() && (files || find) {
            let mut intent = Intent::target(Action::FindLargeFiles, dir);
            intent.minimum_size_bytes = threshold;
            intent.file_type = lexicon::file_type(flat);
            intent.max_age_days = lexicon::age_days(flat)?;
            return Ok(Some(intent));
        }
        if big && (files || find) {
            let mut intent = Intent::target(Action::FindLargeFiles, dir);
            intent.minimum_size_bytes = Some(0);
            intent.file_type = lexicon::file_type(flat);
            intent.max_age_days = lexicon::age_days(flat)?;
            return Ok(Some(intent));
        }
        if size {
            return Ok(Some(Intent::target(Action::DirectorySize, dir)));
        }
    }

    if (first == Some(Verb::Size) || ["size", "du"].contains(&first_raw))
        && w.len() > 1
        && let Some(target) = size_target(strip_leading_openers(&rest(text, 0)))
    {
        return Ok(Some(Intent::target(Action::DirectorySize, target)));
    }

    if first == Some(Verb::Clear) && has(&["кеш", "кэш", "cache"]) {
        for cache in [
            "pip",
            "npm",
            "yarn",
            "pnpm",
            "brew",
            "homebrew",
            "__pycache__",
        ] {
            if has(&[cache]) {
                return Ok(Some(Intent::target(Action::ClearCache, cache)));
            }
        }
    }

    if first == Some(Verb::List) {
        let object = lexicon::strip_object_filler(&rest(text, 1));
        let dir = strip_list_nouns(&object);
        return Ok(Some(if dir.is_empty() {
            Intent::new(Action::ListDirectory)
        } else {
            Intent::target(Action::ListDirectory, dir)
        }));
    }

    if first == Some(Verb::Read) {
        let target = lexicon::strip_object_filler(&rest(text, 1));
        if !target.is_empty() && !lexicon::is_vague_object(&target) {
            return Ok(Some(Intent::target(Action::ReadFile, target)));
        }
    }

    if first == Some(Verb::Create) {
        let target = lexicon::strip_object_filler(&rest(text, 1));
        if !target.is_empty() && !lexicon::is_vague_object(&target) {
            return Ok(Some(Intent::target(Action::CreateDirectory, target)));
        }
    }

    if first == Some(Verb::Copy)
        && let Some(intent) = copy_or_relocate(text, &w, Action::CopyFile)
    {
        return Ok(Some(intent));
    }

    if first == Some(Verb::Cd) || (first_raw == "go" && has(&["to", "in", "into", "в", "на"])) {
        let object = rest(text, 1);
        let dir = object
            .split_whitespace()
            .filter(|word| !CD_FILLERS.contains(&word.to_lowercase().as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        if dir.is_empty() {
            return Ok(Some(Intent::target(Action::ChangeDirectory, "~")));
        }
        return Ok(Some(Intent::target(Action::ChangeDirectory, dir)));
    }

    if matches!(first, Some(Verb::Move) | Some(Verb::Rename)) {
        let separator = w
            .iter()
            .position(|token| RELOCATION_SEPARATORS.contains(&token.as_str()));
        if let Some(separator) = separator
            && separator > 1
            && separator + 1 < w.len()
        {
            let mut intent = Intent::new(if first == Some(Verb::Rename) {
                Action::RenameFile
            } else {
                Action::MoveFile
            });
            intent.source = Some(
                text.split_whitespace()
                    .skip(1)
                    .take(separator - 1)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            intent.destination = Some(rest(text, separator + 1));
            return Ok(Some(intent));
        }
    }

    if !matches!(
        first,
        Some(Verb::Delete)
            | Some(Verb::Kill)
            | Some(Verb::Move)
            | Some(Verb::Rename)
            | Some(Verb::Clear)
    ) && let Some(name) = lexicon::config_tool(text)
    {
        let action = match first {
            Some(Verb::Open) => Action::OpenFile,
            Some(Verb::Read) => Action::ReadFile,
            _ => Action::FindFile,
        };
        return Ok(Some(Intent::target(action, name)));
    }

    if first == Some(Verb::Open) {
        let target = lexicon::strip_app_prefix(&lexicon::strip_object_filler(&rest(text, 1)));
        if !target.is_empty() {
            if lexicon::is_vague_object(&target) {
                return Ok(None);
            }
            let file = target.contains('/')
                || target.contains('.')
                || target.eq_ignore_ascii_case("hosts")
                || lexicon::looks_like_url(&target);
            return Ok(Some(Intent::target(
                if file {
                    Action::OpenFile
                } else {
                    Action::OpenApp
                },
                target,
            )));
        }
    }

    if first == Some(Verb::Delete) || lower.starts_with("get rid of ") {
        let skip = if lower.starts_with("get rid of ") {
            3
        } else {
            1
        };
        let raw_target = rest(text, skip);
        let directory = raw_target
            .split_whitespace()
            .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
            .any(|word| {
                [
                    "папку",
                    "папка",
                    "каталог",
                    "директорию",
                    "folder",
                    "directory",
                    "dir",
                    "carpeta",
                    "ordner",
                    "dossier",
                    "pasta",
                    "cartella",
                    "klasör",
                    "map",
                ]
                .contains(&word.to_lowercase().as_str())
            });
        let target = lexicon::strip_app_prefix(&lexicon::strip_object_filler(&raw_target));
        if !target.is_empty() {
            if lexicon::is_vague_object(&target) {
                return Ok(None);
            }
            let action = if directory {
                Action::DeleteDirectory
            } else if target.contains('/') || target.contains('.') || target == "node_modules" {
                Action::DeleteFile
            } else {
                Action::RemoveApp
            };
            return Ok(Some(Intent::target(action, target)));
        }
    }

    if first == Some(Verb::Find) || lower.starts_with("where is ") {
        let skip = if lower.starts_with("where is ") { 2 } else { 1 };
        let raw_object = rest(text, skip);
        let target = lexicon::strip_object_filler(&raw_object);
        if target.is_empty() {
            if looks_like_listing(&raw_object) {
                return Ok(Some(Intent::new(Action::ListDirectory)));
            }
            return Ok(None);
        }
        if target.contains('.') || target.contains('/') {
            return Ok(Some(Intent::target(Action::FindFile, target)));
        }
        if looks_like_listing(&raw_object) || looks_like_listing(&target) {
            let dir = strip_list_nouns(&target);
            return Ok(Some(if dir.is_empty() {
                Intent::new(Action::ListDirectory)
            } else {
                Intent::target(Action::ListDirectory, dir)
            }));
        }
        if lexicon::is_vague_object(&target) {
            return Ok(None);
        }
        let lowered = target.to_lowercase();
        if lowered.starts_with("процессы ")
            || lowered.starts_with("проццесы ")
            || lowered.starts_with("processes ")
            || lowered.starts_with("process ")
            || target
                .split_whitespace()
                .next()
                .is_some_and(|word| lexicon::edit1(&word.to_lowercase(), "процессы"))
        {
            return Ok(Some(Intent::target(Action::FindProcess, rest(&target, 1))));
        }
        if lowered.starts_with("приложение ") || lowered.starts_with("app ") {
            return Ok(Some(Intent::target(
                Action::FindApp,
                lexicon::strip_app_prefix(&target),
            )));
        }
        if target.split_whitespace().count() != 1 {
            return Ok(None);
        }
        return Ok(Some(Intent::target(Action::FindProcess, target)));
    }

    if first == Some(Verb::Kill) {
        let target = lexicon::strip_app_prefix(&lexicon::strip_object_filler(&rest(text, 1)));
        if !target.is_empty() {
            if lexicon::is_vague_object(&target) {
                return Ok(None);
            }
            if !target.contains('/') && !target.contains('.') {
                return Ok(Some(Intent::target(Action::QuitApp, target)));
            }
        }
    }

    Ok(None)
}

/// Safe fallback for phrasings the strict rules do not cover. Only read-only
/// and low-risk actions are ever produced here, and every target must already
/// resolve to something that exists.
fn loose(text: &str, flat: &str) -> Option<Intent> {
    let tokens = words(text);
    let mut found = Vec::new();
    for token in &tokens {
        if let Some(verb) = lexicon::verb(token)
            && !found.contains(&verb)
        {
            found.push(verb);
        }
    }
    let find = found.contains(&Verb::Find);
    let open = found.contains(&Verb::Open);
    let size = found.contains(&Verb::Size);
    if !found
        .iter()
        .all(|verb| matches!(verb, Verb::Find | Verb::Open | Verb::Size))
    {
        return None;
    }
    if found.is_empty() {
        return None;
    }
    let disk = ["место на диске", "диск", "disk", "свободн", "free space"]
        .iter()
        .any(|marker| flat.contains(marker));
    if disk && (find || size) {
        return Some(Intent::new(Action::ShowDiskUsage));
    }
    if let Some(dir) = lexicon::canonical_dir(flat) {
        if size {
            return Some(Intent::target(Action::DirectorySize, dir));
        }
        if open {
            return Some(Intent::target(Action::OpenFile, dir));
        }
        if find {
            return Some(Intent::target(Action::ListDirectory, dir));
        }
    }
    let index = tokens
        .iter()
        .position(|token| lexicon::verb(token).is_some())?;
    let object = lexicon::strip_object_filler(&rest(text, index + 1));
    if object.is_empty() {
        return None;
    }
    if size && let Some(target) = size_target(strip_leading_openers(&rest(text, index))) {
        return Some(Intent::target(Action::DirectorySize, target));
    }
    if open || find {
        if let Ok(apps) = system::find_apps(&object)
            && !apps.is_empty()
        {
            return Some(Intent::target(
                if find {
                    Action::FindApp
                } else {
                    Action::OpenApp
                },
                object,
            ));
        }
        if let Ok(files) = system::find_files(&object)
            && !files.is_empty()
        {
            return Some(Intent::target(
                if find {
                    Action::FindFile
                } else {
                    Action::OpenFile
                },
                object,
            ));
        }
        if find && system::executable(&object).is_some() {
            return Some(Intent::target(Action::FindProcess, object));
        }
    }
    None
}

fn strip_numbered_prefix(input: &str) -> &str {
    let trimmed = input.trim_start();
    let Some((first, rest)) = trimmed.split_once(char::is_whitespace) else {
        return trimmed;
    };
    let number = first.trim_end_matches(['.', ')', ':']);
    if number != first && !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
        rest.trim_start()
    } else {
        trimmed
    }
}

fn run_tool(program: &str, args: &[&str]) -> Intent {
    let mut intent = Intent::target(Action::RunTool, program);
    intent.argv = Some(args.iter().map(|value| (*value).to_string()).collect());
    intent
}

/// App updates are deliberately opt-in per application. This keeps natural
/// language from turning into an arbitrary package-manager or shell command.
fn natural_app_update(text: &str) -> Option<Intent> {
    let normalized = lexicon::normalize_text(text);
    matches!(
        normalized.as_str(),
        "обнови opencode"
            | "обновить opencode"
            | "обнови open code"
            | "обновить open code"
            | "обнови опенкод"
            | "обновить опенкод"
            | "update opencode"
            | "upgrade opencode"
            | "update open code"
            | "upgrade open code"
            | "opencode update"
            | "opencode upgrade"
    )
    .then(|| Intent::target(Action::UpdateApp, "opencode"))
}

fn natural_git_request(text: &str) -> Option<Intent> {
    let lower = text.to_lowercase();
    let mentions_git = lower.contains("git")
        || lower.contains("коммит")
        || lower.contains("ветк")
        || lower.contains("commit")
        || lower.contains("branch")
        || lower.contains("репозитор")
        || lower.contains("repository")
        || lower.contains(" repo");
    let asks_changed_files =
        (lower.contains("измен") || lower.contains("changed") || lower.contains("modified"))
            && (lower.contains("файл")
                || lower.contains("file")
                || lower.contains("репозитор")
                || lower.contains("repository"));
    if !mentions_git && !asks_changed_files {
        return None;
    }
    if (lower.contains("последн") || lower.contains("latest") || lower.contains("last"))
        && (lower.contains("коммит") || lower.contains("commit"))
    {
        let count = words(text)
            .iter()
            .find_map(|word| word.parse::<u16>().ok())
            .unwrap_or(10)
            .clamp(1, 100)
            .to_string();
        let mut intent = Intent::target(Action::RunTool, "git");
        intent.argv = Some(vec!["log".into(), "-n".into(), count, "--oneline".into()]);
        return Some(intent);
    }
    if (lower.contains("текущ") || lower.contains("сейчас") || lower.contains("current"))
        && (lower.contains("ветк") || lower.contains("branch"))
    {
        return Some(run_tool("git", &["rev-parse", "--abbrev-ref", "HEAD"]));
    }
    if asks_changed_files {
        return Some(run_tool("git", &["status", "--short"]));
    }
    None
}

fn requested_extension(text: &str) -> Option<String> {
    for raw in text.split_whitespace() {
        let candidate = raw
            .trim_matches(|c: char| ",!?;:()[]{}\"'".contains(c))
            .strip_prefix('.');
        if let Some(extension) = candidate
            && !extension.is_empty()
            && extension.len() <= 16
            && extension.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Some(extension.to_lowercase());
        }
    }
    let tokens = words(text);
    for (index, token) in tokens.iter().enumerate() {
        let is_marker = token.starts_with("расширен") || token == "extension";
        let candidate = if token == "extension" {
            index
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
        } else if is_marker {
            tokens.get(index + 1)
        } else if tokens
            .get(index + 1)
            .is_some_and(|next| next == "extension")
        {
            Some(token)
        } else {
            None
        };
        let Some(candidate) = candidate else {
            continue;
        };
        let extension = candidate.trim_start_matches('.');
        if !extension.is_empty()
            && extension.len() <= 16
            && extension.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Some(extension.to_lowercase());
        }
    }
    None
}

fn current_project_file_name(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let tokens = words(text);
    let in_current_scope = [
        "в текущем проекте",
        "в этом проекте",
        "в текущей папке",
        "in the current project",
        "in this project",
        "in the current directory",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || tokens
            .iter()
            .any(|token| matches!(token.as_str(), "здесь" | "here"));
    if !in_current_scope || requested_extension(text).is_some() {
        return None;
    }
    let scope_markers = [
        " в текущем проекте",
        " в этом проекте",
        " в текущей папке",
        " in the current project",
        " in this project",
        " in the current directory",
    ];
    let start_markers = [
        "покажи где лежит ",
        "покажи, где лежит ",
        "search for ",
        "show me where ",
        "find где лежит ",
        "find for ",
        "find me where ",
        "найди ",
        "поищи ",
        "find ",
        "locate ",
    ];
    for start in start_markers {
        let Some(remainder) = lower.strip_prefix(start) else {
            continue;
        };
        let Some((scope_index, _)) = scope_markers
            .iter()
            .filter_map(|marker| remainder.find(marker).map(|index| (index, marker)))
            .min_by_key(|(index, _)| *index)
        else {
            continue;
        };
        let mut query = remainder[..scope_index].trim();
        if start == "show me where " || start == "find me where " {
            query = query.strip_suffix(" is").unwrap_or(query).trim();
        }
        if !query.is_empty() {
            return Some(query.to_string());
        }
    }
    let verb = tokens
        .iter()
        .position(|token| lexicon::verb(token) == Some(Verb::Find))?;
    let drop = [
        "all",
        "все",
        "файл",
        "файлы",
        "file",
        "files",
        "here",
        "здесь",
    ];
    let mut kept = Vec::new();
    for token in tokens.iter().skip(verb + 1) {
        if matches!(token.as_str(), "в" | "in") {
            break;
        }
        if !drop.contains(&token.as_str()) {
            kept.push(token.as_str());
        }
    }
    let query = kept.join(" ");
    (!query.is_empty()).then_some(query)
}

fn natural_contextual_request(text: &str, flat: &str) -> Option<Intent> {
    let lower = text.to_lowercase();
    let current_scope = [
        "текущая папка",
        "текущей папке",
        "текущий проект",
        "текущем проекте",
        "current folder",
        "current directory",
        "current project",
        "this project",
    ]
    .iter()
    .any(|marker| lower.contains(marker));

    if current_scope
        && ["сколько весит", "размер", "size", "how big", "how large"]
            .iter()
            .any(|marker| lower.contains(marker))
    {
        return Some(Intent::target(Action::DirectorySize, "."));
    }

    if let Some(extension) = requested_extension(text)
        && (current_scope || lower.contains("здесь") || lower.contains(" here"))
    {
        let mut intent = Intent::target(Action::FindFiles, ".");
        intent.file_extension = Some(extension);
        intent.limit = Some(500);
        return Some(intent);
    }

    if let Some(name) = current_project_file_name(text) {
        let mut intent = Intent::target(Action::FindFiles, ".");
        intent.name_contains = Some(name);
        intent.limit = Some(500);
        return Some(intent);
    }

    let tokens = words(text);
    let pid_question = (lower.contains("pid")
        && [
            "что ",
            "какой ",
            "покажи ",
            "кому ",
            "who ",
            "which ",
            "identify ",
            "details ",
        ]
        .iter()
        .any(|marker| lower.starts_with(marker)))
        || lower.contains("детали процесса")
        || lower.contains("details for process");
    if pid_question
        && !lower.contains("порт")
        && !lower.contains("port")
        && let Some(pid) = tokens.iter().find_map(|token| token.parse::<u32>().ok())
    {
        let mut intent = Intent::new(Action::FindProcess);
        intent.pid = Some(pid);
        return Some(intent);
    }

    let heavy = [
        "больше всего места",
        "самое тяжел",
        "самые тяжел",
        "самое тяжёл",
        "самые тяжёл",
        "жрет место",
        "жрёт место",
        "takes the most space",
        "largest files",
        "heaviest files",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if heavy {
        if let Some(directory) = lexicon::canonical_dir(&lower) {
            let mut intent = Intent::target(Action::FindLargeFiles, directory);
            intent.minimum_size_bytes = Some(0);
            return Some(intent);
        }
        if lower.contains("место на диске")
            || lower.contains("пропало место")
            || lower.contains("disk space")
        {
            let mut intent = Intent::target(Action::FindLargeFiles, "~");
            intent.minimum_size_bytes = Some(0);
            return Some(intent);
        }
    }

    if (lower.contains("процесс") || lower.contains("process"))
        && (lower.contains("связан") || lower.contains("related"))
    {
        let app = system::mentioned_app(flat).or_else(|| {
            text.split_whitespace()
                .next_back()
                .map(|value| {
                    value
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string()
                })
                .filter(|value| !value.is_empty())
        });
        if let Some(app) = app {
            return Some(Intent::target(Action::FindProcess, app));
        }
    }

    if (lower.contains("браузер") || lower.contains("browser"))
        && (lower.contains("запущ")
            || lower.contains("running")
            || lower.contains("открыт")
            || lower.contains("работающ")
            || lower.contains(" open ")
            || lower.starts_with("show open"))
    {
        return Some(Intent::target(Action::ListProcesses, "browser"));
    }

    if (lower.contains("мак тормозит")
        || lower.contains("mac тормозит")
        || lower.contains("компьютер тормозит")
        || lower.contains("мак медлен")
        || lower.contains("mac is slow")
        || lower.contains("mac so slow")
        || lower.contains("computer is slow")
        || lower.contains("slow computer"))
        && (lower.contains("почему")
            || lower.contains("why")
            || lower.contains("понять")
            || lower.contains("diagnose"))
    {
        return Some(Intent::target(Action::ListProcesses, "diagnostic"));
    }

    if lower.contains("съел всю оперативку")
        || lower.contains("съело всю оперативку")
        || lower.contains("eating all the memory")
    {
        return Some(Intent::target(Action::ListProcesses, "memory"));
    }

    if (lower.contains("приложен") || lower.contains(" app ") || lower.starts_with("find app "))
        && lexicon::verb(words(text).first()?.as_str()) == Some(Verb::Find)
        && !lower.contains("процесс")
        && let Some(app) = system::mentioned_app(flat)
    {
        return Some(Intent::target(Action::FindApp, app));
    }
    None
}

fn prepared(input: &str) -> String {
    let aliased = crate::alias::rewrite(strip_numbered_prefix(input));
    let expanded = crate::knowledge::rewrite(&aliased);
    let normalized_spacing = expanded.split_whitespace().collect::<Vec<_>>().join(" ");
    lexicon::rewrite_leading(&lexicon::strip_prefixes(&normalized_spacing))
}

fn phrase_key(input: &str) -> String {
    input
        .trim()
        .trim_matches(|character: char| ".,!?;:".contains(character))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn exact_read_only_action(input: &str) -> Option<Action> {
    let key = phrase_key(input);
    if DISK_PHRASES.contains(&key.as_str())
        || matches!(
            key.as_str(),
            "am i about to run out of storage" | "сколько осталось места на маке"
        )
    {
        return Some(Action::ShowDiskUsage);
    }
    if matches!(
        key.as_str(),
        "battery level"
            | "battery charge"
            | "battery percentage"
            | "сколько у меня зарядки"
            | "покажи заряд батареи"
            | "заряд батареи"
            | "hoeveel batterij heb ik nog"
            | "كم تبقى من البطارية"
            | "berapa persen baterai"
    ) {
        return Some(Action::ShowBattery);
    }
    if matches!(
        key.as_str(),
        "what's the time"
            | "whats the time"
            | "what time is it"
            | "tell me when it is"
            | "what day is it"
            | "какой сейчас день"
    ) {
        return Some(Action::ShowDate);
    }
    if key == "how long has this laptop been awake" {
        return Some(Action::ShowUptime);
    }
    if matches!(
        key.as_str(),
        "what is this machine called" | "что за имя у этого мака"
    ) {
        return Some(Action::ShowHostname);
    }
    [
        (SYSTEM_INFO_PHRASES, Action::ShowSystemInfo),
        (UPTIME_PHRASES, Action::ShowUptime),
        (MEMORY_PHRASES, Action::ShowMemory),
        (CPU_PHRASES, Action::ShowCpu),
        (NETWORK_PHRASES, Action::ShowNetwork),
    ]
    .into_iter()
    .find_map(|(phrases, action)| phrases.contains(&key.as_str()).then_some(action))
}

fn unsafe_shell_syntax(input: &str) -> bool {
    [
        "\n", "\r", "\0", "`", "$(", ";", " | ", " > ", " < ", " && ", " || ",
    ]
    .iter()
    .any(|marker| input.contains(marker))
}

fn developer_tool_shaped(input: &str) -> bool {
    input
        .split_whitespace()
        .next()
        .is_some_and(|word| matches!(word.to_lowercase().as_str(), "git" | "brew" | "cargo"))
}

fn obvious_non_command_prose(input: &str) -> bool {
    let lower = input.trim().to_lowercase();
    [
        "i read about ",
        "i was reading about ",
        "я читал про ",
        "я читала про ",
        "port 3000 is commonly used ",
        "port 8080 is commonly used ",
        "port 3000 is usually used ",
        "port 8080 is usually used ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

/// Strict rules only.
pub fn parse(input: &str) -> Option<Intent> {
    if input.len() > MAX_INPUT_BYTES {
        return None;
    }
    if unsafe_shell_syntax(input) || obvious_non_command_prose(input) {
        return None;
    }
    if has_multiple_steps(input) {
        return None;
    }
    if let Some(action) = exact_read_only_action(input) {
        return Some(Intent::new(action));
    }
    if let Some(intent) = complete_app_removal(input) {
        return Some(intent);
    }
    let text = prepared(input);
    let flat = lexicon::normalize_text(&text);
    if explicitly_negated_action(&flat) {
        return None;
    }
    if let Some(intent) = complete_app_removal(&text) {
        return Some(intent);
    }
    if let Some(intent) = http_get_request(&text).ok().flatten() {
        return Some(intent);
    }
    if let Some(intent) = natural_git_request(&text) {
        return Some(intent);
    }
    if let Some(intent) = natural_app_update(&text) {
        return Some(intent);
    }
    if let Some(intent) = natural_contextual_request(&text, &flat) {
        return Some(intent);
    }
    if lexicon::unsupported_goal(&flat).is_some() {
        return None;
    }
    if let Some(intent) = crate::commands::parse(&text) {
        return Some(intent);
    }
    if let Some(intent) = crate::tools::parse(&text) {
        return Some(intent);
    }
    if developer_tool_shaped(&text) {
        return None;
    }
    strict(&text, &flat).ok().flatten()
}

/// Full fast path: unsupported goals, strict rules, then the safe fallback.
pub fn parse_outcome(input: &str) -> Option<Parsed> {
    if input.len() > MAX_INPUT_BYTES {
        return Some(Parsed::Refused(format!(
            "Request is longer than {MAX_INPUT_BYTES} bytes. No changes made."
        )));
    }
    if unsafe_shell_syntax(input) {
        return Some(Parsed::Refused(
            "Shell operators are not accepted. Nothing was executed.".into(),
        ));
    }
    if obvious_non_command_prose(input) {
        return None;
    }
    if has_multiple_steps(input) {
        return None;
    }
    if let Some(action) = exact_read_only_action(input) {
        return Some(Parsed::Intent(Intent::new(action)));
    }
    if let Some(intent) = complete_app_removal(input) {
        return Some(Parsed::Intent(intent));
    }
    let text = prepared(input);
    let flat = lexicon::normalize_text(&text);
    if explicitly_negated_action(&flat) {
        return Some(Parsed::Refused(
            "The request is explicitly negated. Nothing was executed.".into(),
        ));
    }
    if let Some(intent) = complete_app_removal(&text) {
        return Some(Parsed::Intent(intent));
    }
    match http_get_request(&text) {
        Ok(Some(intent)) => return Some(Parsed::Intent(intent)),
        Err(reason) => return Some(Parsed::Refused(reason)),
        Ok(None) => {}
    }
    if let Some(intent) = natural_git_request(&text) {
        return Some(Parsed::Intent(intent));
    }
    if let Some(intent) = natural_app_update(&text) {
        return Some(Parsed::Intent(intent));
    }
    if let Some(intent) = natural_contextual_request(&text, &flat) {
        return Some(Parsed::Intent(intent));
    }
    if let Some(intent) = crate::commands::parse(&text) {
        return Some(Parsed::Intent(intent));
    }
    if let Some(intent) = crate::tools::parse(&text) {
        return Some(Parsed::Intent(intent));
    }
    if developer_tool_shaped(&text) {
        return Some(Parsed::Refused(
            "That developer-tool command is not allowlisted. Nothing was executed.".into(),
        ));
    }
    if let Some(goal) = lexicon::unsupported_goal(&flat) {
        let target = match goal {
            crate::intent::UnsupportedGoal::SetDefaultBrowser
            | crate::intent::UnsupportedGoal::EnableLoginItem
            | crate::intent::UnsupportedGoal::DisableLoginItem => system::mentioned_app(&flat),
            _ => None,
        };
        return Some(Parsed::Unsupported(SemanticGoal {
            action: "UNSUPPORTED",
            semantic_goal: goal,
            target,
            confidence: 1.0,
        }));
    }
    if let Some(intent) = conversational(&text, &flat) {
        return Some(Parsed::Intent(intent));
    }
    match strict(&text, &flat) {
        Ok(Some(intent)) => Some(Parsed::Intent(intent)),
        Err(reason) => Some(Parsed::Refused(reason)),
        Ok(None) => loose(&text, &flat)
            .or_else(|| crate::frame::parse(&text))
            .or_else(|| implied_open(&text))
            .or_else(|| {
                lexicon::config_tool(&text).map(|name| Intent::target(Action::FindFile, name))
            })
            .map(Parsed::Intent),
    }
}

fn listing_phrase_dir(lower: &str, text: &str) -> Option<String> {
    const PREFIXES: &[&str] = &[
        "what's in ",
        "whats in ",
        "what is in ",
        "что в ",
        "что внутри ",
        "qué hay en ",
        "que hay en ",
        "was ist in ",
        "qu'y a-t-il dans ",
    ];
    for prefix in PREFIXES {
        if let Some(tail) = lower.strip_prefix(prefix) {
            let skip = text.split_whitespace().count() - tail.split_whitespace().count();
            return Some(strip_list_nouns(&rest(text, skip)));
        }
    }
    None
}

fn is_pwd_request(first: &str, words: &[String], flat: &str) -> bool {
    matches!(
        flat,
        "где я"
            | "where am i"
            | "current directory"
            | "working directory"
            | "текущая папка"
            | "текущая директория"
            | "dónde estoy"
            | "donde estoy"
            | "wo bin ich"
            | "où suis je"
            | "ou suis je"
    ) || (first == "где" && words.get(1).map(String::as_str) == Some("я"))
        || (first == "where" && words.get(1).map(String::as_str) == Some("am"))
}

fn looks_like_listing(object: &str) -> bool {
    if object.contains('.') {
        return false;
    }
    const MARKERS: &[&str] = &[
        "файл",
        "files",
        "file",
        "archivos",
        "dateien",
        "fichiers",
        "файли",
        "содержим",
        "content",
        "contenid",
        "inhalt",
    ];
    object.split(|c: char| !c.is_alphanumeric()).any(|word| {
        let word = word.to_lowercase();
        MARKERS.iter().any(|marker| word.starts_with(marker))
    })
}

fn strip_list_nouns(object: &str) -> String {
    const DROP: &[&str] = &[
        "files",
        "file",
        "файлы",
        "файлов",
        "файли",
        "archivos",
        "dateien",
        "fichiers",
        "in",
        "в",
        "on",
        "на",
        "the",
        "folder",
        "folders",
        "папке",
        "папки",
        "папку",
        "directory",
        "dir",
        "here",
        "тут",
        "здесь",
        "contenido",
        "contents",
        "содержимое",
    ];
    object
        .split_whitespace()
        .filter(|word| !DROP.contains(&word.to_lowercase().as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn copy_or_relocate(text: &str, words: &[String], action: Action) -> Option<Intent> {
    let separator = words
        .iter()
        .position(|token| RELOCATION_SEPARATORS.contains(&token.as_str()));
    if let Some(separator) = separator
        && separator > 1
        && separator + 1 < words.len()
    {
        let mut intent = Intent::new(action);
        intent.source = Some(
            text.split_whitespace()
                .skip(1)
                .take(separator - 1)
                .collect::<Vec<_>>()
                .join(" "),
        );
        intent.destination = Some(rest(text, separator + 1));
        return Some(intent);
    }
    if words.len() == 3 {
        let mut intent = Intent::new(action);
        intent.source = Some(text.split_whitespace().nth(1)?.to_string());
        intent.destination = Some(rest(text, 2));
        return Some(intent);
    }
    None
}

fn implied_open(text: &str) -> Option<Intent> {
    let object = lexicon::strip_object_filler(text.trim());
    if object.is_empty() || lexicon::is_vague_object(&object) {
        return None;
    }
    if object.split_whitespace().count() > 4 {
        return None;
    }
    if lexicon::looks_like_url(&object) {
        return Some(Intent::target(Action::OpenFile, object));
    }
    if let Ok(apps) = system::find_apps(&object)
        && !apps.is_empty()
    {
        return Some(Intent::target(Action::OpenApp, object));
    }
    if system::existing_dir(&object).is_some() {
        return Some(Intent::target(Action::OpenFile, object));
    }
    if system::checked_existing(&object).is_ok() {
        return Some(Intent::target(Action::OpenFile, object));
    }
    if let Ok(files) = system::find_files(&object)
        && !files.is_empty()
    {
        return Some(Intent::target(Action::OpenFile, object));
    }
    None
}

pub fn has_multiple_steps(input: &str) -> bool {
    let lower = input.to_lowercase();
    [" и ", " потом ", " and then ", " then ", " а потом "]
        .iter()
        .any(|separator| lower.contains(separator))
        || (lower.contains(" and ")
            && ["remove ", "uninstall ", "delete ", "get rid of "]
                .iter()
                .any(|verb| lower.trim_start().starts_with(verb)))
}

pub fn parse_multi_step(input: &str) -> Option<Vec<Intent>> {
    let input = input.trim();
    if unsafe_shell_syntax(input) {
        return None;
    }
    let lower = input.to_lowercase();
    let separator = [" и ", " and then ", " then ", " а потом ", " and "]
        .into_iter()
        .find(|separator| lower.contains(separator))?;
    let split = lower.find(separator)?;
    let before = &input[..split];
    let after = &input[split + separator.len()..];
    let second_text = after.to_lowercase();
    let before_lower = before.to_lowercase();
    let first = parse(before)?;

    // A single destructive verb can govern a short list of application names:
    // "удали Roblox и Roblox Studio" / "remove VLC and Zoom". Keep this
    // deliberately limited to app removal. Both targets are resolved and
    // preflighted by run_multi_step before either one is changed.
    if first.action == Action::RemoveApp {
        if let Some(second) = parse(after) {
            if second.action == Action::RemoveApp {
                return Some(vec![first, second]);
            }
            return None;
        }
        let target = lexicon::strip_app_prefix(&lexicon::strip_object_filler(after));
        let plain_name = !target.is_empty()
            && !lexicon::is_vague_object(&target)
            && target.split_whitespace().count() <= 8
            && target.chars().all(|character| {
                character.is_alphanumeric()
                    || character.is_whitespace()
                    || matches!(character, '-' | '_' | '.' | '+' | '&' | '\'' | '(' | ')')
            });
        if plain_name {
            return Some(vec![first, Intent::target(Action::RemoveApp, target)]);
        }
        return None;
    }

    if (before_lower.starts_with("find ") || before_lower.starts_with("найди "))
        && ["uninstall it", "remove it", "удали его", "снеси его"].contains(&second_text.trim())
    {
        let target = rest(before, 1);
        if !target.is_empty() && !target.contains(['/', '.']) {
            return Some(vec![
                Intent::target(Action::FindApp, &target),
                Intent::target(Action::RemoveApp, target),
            ]);
        }
    }
    if before_lower == "открой hosts" && second_text.starts_with("запусти ") {
        let target = rest(after, 1);
        if !target.is_empty() && !target.contains(['/', '.']) {
            return Some(vec![
                Intent::target(Action::OpenFile, "hosts"),
                Intent::target(Action::OpenApp, target),
            ]);
        }
    }
    if first.action == Action::FindPortProcess
        && [
            "выключи",
            "убей",
            "kill",
            "stop",
            "free",
            "освободи",
            "закрой",
        ]
        .iter()
        .any(|verb| second_text.contains(verb))
        && ["его", "её", "it", "that"].iter().any(|reference| {
            second_text
                .split_whitespace()
                .any(|word| word == *reference)
        })
    {
        let mut second = Intent::new(Action::KillPortProcess);
        second.port = first.port;
        return Some(vec![first, second]);
    }
    if first.action == Action::KillPortProcess
        && (second_text.contains("проверь порт") || second_text.contains("verify port"))
    {
        let mut second = Intent::new(Action::FindPortProcess);
        second.port = first.port;
        return Some(vec![first, second]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(phrase: &str) -> Action {
        parse(phrase)
            .unwrap_or_else(|| panic!("no fast match for {phrase}"))
            .action
    }

    #[test]
    fn common_phrases() {
        assert_eq!(action("сколько у меня зарядки"), Action::ShowBattery);
        assert_eq!(action("найди работающие процессы"), Action::ListProcesses);
        assert_eq!(parse("сколько стоит ремонт").map(|i| i.action), None);
        let bluetooth = parse("включи блютуз").unwrap();
        assert_eq!(bluetooth.action, Action::SetBluetoothPower);
        assert_eq!(bluetooth.target.as_deref(), Some("on"));
        assert_eq!(
            parse("выключи bluetooth").unwrap().target.as_deref(),
            Some("off")
        );
        for phrase in [
            "выключи блютус",
            "выключи блутуз",
            "выключи блутус",
            "disable bluetoth",
            "turn off blue tooth",
            "turn the Bluetooth off",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::SetBluetoothPower, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("off"), "{phrase}");
        }
        for phrase in [
            "включи блютус",
            "enable bluetooh",
            "turn on blue tooth",
            "turn the Bluetooth on",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::SetBluetoothPower, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("on"), "{phrase}");
        }
        for phrase in [
            "удали Chrome",
            "снеси Chrome",
            "убери Chrome",
            "выкинь Chrome",
            "хочу удалить Chrome",
            "remove Chrome",
            "uninstall Chrome",
            "get rid of Chrome",
            "I would like to remove Google Chrome",
        ] {
            assert_eq!(action(phrase), Action::RemoveApp, "{phrase}");
        }
        for phrase in [
            "полностью удали Zoom",
            "удали полностью Zoom",
            "completely uninstall Firefox",
            "fully remove VLC",
        ] {
            assert_eq!(action(phrase), Action::RemoveAppCompletely, "{phrase}");
        }
        for phrase in [
            "обнови opencode",
            "обновить OpenCode",
            "обнови опенкод",
            "update opencode",
            "opencode upgrade",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::UpdateApp, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("opencode"), "{phrase}");
        }
        assert!(parse("как обновить opencode").is_none());
        for phrase in [
            "почему интернет тормозит",
            "интернет медленный",
            "diagnose my slow internet",
            "why is my network slow",
        ] {
            assert_eq!(action(phrase), Action::DiagnoseNetwork, "{phrase}");
        }
        for phrase in [
            "кто на 8765",
            "кто сидит на 8765",
            "что занимает 8765",
            "что держит порт 8765",
            "какой процесс слушает 8765",
            "who uses port 8765",
            "what is listening on 8765",
            "what's on port 8765",
        ] {
            assert_eq!(action(phrase), Action::FindPortProcess, "{phrase}");
        }
        for phrase in [
            "убей процесс на 8765",
            "освободи порт 8765",
            "выключи то что сидит на 8765",
            "kill process on port 8765",
            "free port 8765",
            "stop whatever is listening on 8765",
            "закрой порт 8765",
        ] {
            assert_eq!(action(phrase), Action::KillPortProcess, "{phrase}");
        }
        assert_eq!(action("сколько весит Downloads"), Action::DirectorySize);
        assert_eq!(action("очисти кеш pip"), Action::ClearCache);
        assert_eq!(
            parse("найди файлы больше 1 гб в Downloads")
                .unwrap()
                .minimum_size_bytes,
            Some(1_073_741_824)
        );
        let direct = parse("kill process 123").unwrap();
        assert_eq!(direct.action, Action::KillProcess);
        assert_eq!(direct.pid, Some(123));
    }

    #[test]
    fn natural_question_regressions_keep_structured_slots() {
        for phrase in [
            "hey tf turn Bluetooth off",
            "please can you turn Bluetooth off",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::SetBluetoothPower, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("off"), "{phrase}");
        }

        for phrase in [
            "кому принадлежит PID 46272",
            "show details for process 46272",
            "identify PID 46272",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::FindProcess, "{phrase}");
            assert_eq!(intent.pid, Some(46272), "{phrase}");
        }
        assert_eq!(action("убей PID 123"), Action::KillProcess);

        for phrase in [
            "что изменилось в репозитории",
            "покажи измененные файлы",
            "which files are modified",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::RunTool, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("git"), "{phrase}");
            assert_eq!(
                intent.argv,
                Some(vec!["status".into(), "--short".into()]),
                "{phrase}"
            );
        }

        for phrase in [
            "покажи где лежит README в этом проекте",
            "search for README in the current project",
            "show me where README is in this project",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::FindFiles, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("."), "{phrase}");
            assert_eq!(intent.name_contains.as_deref(), Some("readme"), "{phrase}");
        }

        let extension = parse("search this project for .json files").unwrap();
        assert_eq!(extension.action, Action::FindFiles);
        assert_eq!(extension.file_extension.as_deref(), Some("json"));
    }

    #[test]
    fn launch_and_close_are_understood() {
        for phrase in [
            "открой telegram",
            "запусти telegram",
            "открой телеграм",
            "открой у меня телеграм пожалуйста",
            "open telegram",
            "launch Safari",
            "запусти приложение Figma",
            "открой терминал",
            "открой настройки",
        ] {
            assert_eq!(action(phrase), Action::OpenApp, "{phrase}");
        }
        for phrase in ["закрой telegram", "quit Safari", "останови Discord"] {
            assert_eq!(action(phrase), Action::QuitApp, "{phrase}");
        }
        for phrase in [
            "открой youtube.com",
            "открой https://github.com",
            "open vk.com",
        ] {
            assert_eq!(action(phrase), Action::OpenFile, "{phrase}");
        }
        assert_eq!(action("открой Downloads"), Action::OpenApp);
        assert_eq!(action("открой ~/Projects"), Action::OpenFile);
    }

    #[test]
    fn directory_requests_are_typed() {
        for (phrase, directory) in [
            ("сколько весит Downloads", "Downloads"),
            ("размер Documents", "Documents"),
            ("size Desktop", "Desktop"),
            ("у меня Downloads занимает много места", "Downloads"),
            ("сколько места занимает Downloads", "Downloads"),
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no fast match for {phrase}"));
            assert_eq!(intent.action, Action::DirectorySize, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some(directory), "{phrase}");
        }
        let intent = parse("найди большие видео за месяц в Downloads").unwrap();
        assert_eq!(intent.action, Action::FindLargeFiles);
        assert_eq!(intent.target.as_deref(), Some("Downloads"));
        assert_eq!(intent.file_type, Some(crate::intent::FileType::Video));
        assert_eq!(intent.max_age_days, Some(30));
        let intent = parse("покажи самые большие файлы в Documents").unwrap();
        assert_eq!(intent.action, Action::FindLargeFiles);
        assert_eq!(intent.target.as_deref(), Some("Documents"));
    }

    #[test]
    fn unsupported_goals_never_execute() {
        for (phrase, goal) in [
            (
                "change default browser to Google Chrome",
                crate::intent::UnsupportedGoal::SetDefaultBrowser,
            ),
            (
                "enable Google Chrome at login",
                crate::intent::UnsupportedGoal::EnableLoginItem,
            ),
            (
                "запрети Discord запускаться при входе",
                crate::intent::UnsupportedGoal::DisableLoginItem,
            ),
            (
                "поставь мне Docker",
                crate::intent::UnsupportedGoal::InstallApp,
            ),
            ("обнови macOS", crate::intent::UnsupportedGoal::UpdateOs),
            (
                "send an email to Alex",
                crate::intent::UnsupportedGoal::SendEmail,
            ),
            (
                "создай новый аккаунт",
                crate::intent::UnsupportedGoal::CreateAccount,
            ),
            (
                "download latest Ubuntu ISO",
                crate::intent::UnsupportedGoal::DownloadFile,
            ),
        ] {
            match parse_outcome(phrase).unwrap_or_else(|| panic!("no match for {phrase}")) {
                Parsed::Unsupported(found) => assert_eq!(found.semantic_goal, goal, "{phrase}"),
                _ => panic!("expected unsupported goal for {phrase}"),
            }
        }
    }

    #[test]
    fn loose_path_is_read_only() {
        for phrase in [
            "снеси приложение",
            "грохни всё подряд",
            "перемести что-нибудь",
            "выпили это",
        ] {
            assert!(parse(phrase).is_none(), "{phrase}");
            match parse_outcome(phrase) {
                None | Some(Parsed::Unsupported(_)) | Some(Parsed::Refused(_)) => {}
                Some(Parsed::Intent(intent)) => assert!(
                    matches!(
                        intent.action,
                        Action::FindApp
                            | Action::OpenApp
                            | Action::FindFile
                            | Action::OpenFile
                            | Action::FindProcess
                            | Action::DirectorySize
                            | Action::FindLargeFiles
                            | Action::ShowDiskUsage
                    ),
                    "loose path produced {:?} for {phrase}",
                    intent.action
                ),
            }
        }
        match parse_outcome("запусти у меня телеграм").unwrap() {
            Parsed::Intent(intent) => {
                assert_eq!(intent.action, Action::OpenApp);
                assert_eq!(intent.target.as_deref(), Some("телеграм"));
            }
            _ => panic!("expected a loose open intent"),
        }
    }

    #[test]
    fn fast_path_covers_the_semantic_fixture() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/intents.json")).unwrap();
        assert!(cases.len() >= 100);
        let mut strict_hits = 0;
        for case in &cases {
            let input = case["input"].as_str().unwrap();
            let expected = case["expected_goal"].as_str().unwrap();
            if expected == "MULTI_STEP" {
                assert!(
                    has_multiple_steps(input),
                    "multi-step marker missing: {input}"
                );
                assert!(parse_outcome(input).is_none(), "multi-step leaked: {input}");
                continue;
            }
            let outcome =
                parse_outcome(input).unwrap_or_else(|| panic!("fast path missed: {input}"));
            let encoded = match outcome {
                Parsed::Intent(intent) => serde_json::to_value(&intent).unwrap(),
                Parsed::Unsupported(goal) => serde_json::to_value(&goal).unwrap(),
                Parsed::Refused(reason) => panic!("fast path refused {input}: {reason}"),
            };
            if let Some(fields) = case
                .get("expected_fields")
                .and_then(|value| value.as_object())
            {
                for (key, expected_value) in fields {
                    assert_eq!(
                        encoded.get(key),
                        Some(expected_value),
                        "field {key} mismatch for {input}"
                    );
                }
            }
            assert_eq!(encoded["action"], expected, "wrong mapping for {input}");
            if case.get("expected_fields").is_none() {
                strict_hits += 1;
            }
        }
        eprintln!("fast parser covered {} fixture cases", strict_hits);
    }

    #[test]
    fn semantic_fixture_fast_path_has_no_false_action_mappings() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/intents.json")).unwrap();
        for case in &cases {
            let input = case["input"].as_str().unwrap();
            let expected = case["expected_goal"].as_str().unwrap();
            if let Some(intent) = parse(input) {
                let encoded = serde_json::to_value(&intent).unwrap();
                assert_eq!(
                    encoded["action"], expected,
                    "unsafe fast mapping for {input}"
                );
            }
        }
    }

    #[test]
    fn every_registered_capability_has_a_fixture() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/intents.json")).unwrap();
        let covered = cases
            .iter()
            .filter_map(|case| case["expected_goal"].as_str())
            .collect::<std::collections::HashSet<_>>();
        for capability in crate::intent::CAPABILITIES {
            assert!(
                covered.contains(capability.name),
                "{} has no end-to-end fixture",
                capability.name
            );
        }
    }

    #[test]
    fn every_documented_example_maps_to_its_capability() {
        for capability in crate::intent::CAPABILITIES {
            for example in capability.examples {
                let intent = parse(example).unwrap_or_else(|| {
                    panic!("documented example has no fast match: tf {example}")
                });
                assert_eq!(
                    intent.action, capability.action,
                    "documented example `tf {example}` maps to {:?}, not {}",
                    intent.action, capability.name
                );
            }
        }
    }

    #[test]
    fn fixtures_tolerate_case_and_whitespace_variations() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/intents.json")).unwrap();
        for case in &cases {
            let input = case["input"].as_str().unwrap();
            let expected = case["expected_goal"].as_str().unwrap();
            if expected == "MULTI_STEP" {
                continue;
            }
            for variant in [
                format!(
                    "  {}  ",
                    input.split_whitespace().collect::<Vec<_>>().join("   ")
                ),
                input.to_uppercase(),
            ] {
                let outcome = parse_outcome(&variant)
                    .unwrap_or_else(|| panic!("variation missed: {variant:?} from {input:?}"));
                let action = match outcome {
                    Parsed::Intent(intent) => serde_json::to_value(intent).unwrap()["action"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    Parsed::Unsupported(_) => "UNSUPPORTED".to_string(),
                    Parsed::Refused(reason) => panic!("variation refused {variant:?}: {reason}"),
                };
                assert_eq!(action, expected, "variation {variant:?} from {input:?}");
            }
        }
    }

    #[test]
    fn system_diagnostics_are_multilingual() {
        for phrase in SYSTEM_INFO_PHRASES {
            assert_eq!(action(phrase), Action::ShowSystemInfo, "{phrase}");
        }
        for phrase in UPTIME_PHRASES {
            assert_eq!(action(phrase), Action::ShowUptime, "{phrase}");
        }
        for phrase in MEMORY_PHRASES {
            assert_eq!(action(phrase), Action::ShowMemory, "{phrase}");
        }
        for phrase in CPU_PHRASES {
            assert_eq!(action(phrase), Action::ShowCpu, "{phrase}");
        }
        for phrase in NETWORK_PHRASES {
            assert_eq!(action(phrase), Action::ShowNetwork, "{phrase}");
        }
    }

    #[test]
    fn multilingual_copy_move_and_rename_preserve_both_paths() {
        let copy_cases = [
            "copy a.txt to b.txt",
            "скопируй a.txt в b.txt",
            "скопіюй a.txt до b.txt",
            "copia a.txt a b.txt",
            "kopiere a.txt nach b.txt",
            "copie a.txt vers b.txt",
            "copie a.txt para b.txt",
            "copia a.txt in b.txt",
            "skopiuj a.txt do b.txt",
            "kopyala a.txt içine b.txt",
            "kopieer a.txt naar b.txt",
            "zkopíruj a.txt do b.txt",
            "复制 a.txt 到 b.txt",
            "コピー a.txt に b.txt",
            "복사 a.txt 에 b.txt",
            "انسخ a.txt إلى b.txt",
            "कॉपी करो a.txt में b.txt",
        ];
        for phrase in copy_cases {
            assert_binary_slots(phrase, Action::CopyFile);
        }
        for phrase in [
            "move a.txt to b.txt",
            "перемести a.txt в b.txt",
            "перемісти a.txt до b.txt",
            "mueve a.txt a b.txt",
            "verschiebe a.txt nach b.txt",
            "déplace a.txt vers b.txt",
            "mova a.txt para b.txt",
            "sposta a.txt in b.txt",
            "przenieś a.txt do b.txt",
            "taşı a.txt içine b.txt",
            "verplaats a.txt naar b.txt",
            "přesuň a.txt do b.txt",
            "移动 a.txt 到 b.txt",
            "انقل a.txt إلى b.txt",
            "ले जाओ a.txt में b.txt",
        ] {
            assert_binary_slots(phrase, Action::MoveFile);
        }
        for phrase in [
            "rename a.txt to b.txt",
            "переименуй a.txt как b.txt",
            "перейменуй a.txt як b.txt",
            "renombra a.txt como b.txt",
            "benenne a.txt als b.txt",
            "renomme a.txt en b.txt",
            "renomeie a.txt como b.txt",
            "rinomina a.txt come b.txt",
            "zmień nazwę a.txt jako b.txt",
            "yeniden adlandır a.txt olarak b.txt",
            "hernoem a.txt als b.txt",
            "přejmenuj a.txt jako b.txt",
            "重命名 a.txt 到 b.txt",
            "أعد تسمية a.txt إلى b.txt",
            "नाम बदलो a.txt में b.txt",
        ] {
            assert_binary_slots(phrase, Action::RenameFile);
        }
    }

    fn assert_binary_slots(phrase: &str, expected: Action) {
        let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
        assert_eq!(intent.action, expected, "{phrase}");
        assert_eq!(intent.source.as_deref(), Some("a.txt"), "{phrase}");
        assert_eq!(intent.destination.as_deref(), Some("b.txt"), "{phrase}");
    }

    #[test]
    fn shell_syntax_never_becomes_a_tool_action() {
        for phrase in [
            "git status; touch /tmp/pwned",
            "git log | sh",
            "cargo test && open /tmp/pwned",
            "brew info $(whoami)",
            "git diff `touch /tmp/pwned`",
            "cargo test > /tmp/output",
        ] {
            assert_ne!(
                parse(phrase).map(|intent| intent.action),
                Some(Action::RunTool),
                "{phrase}"
            );
        }
    }

    #[test]
    fn expanded_scenario_regressions_are_safe_and_deterministic() {
        for phrase in [
            "git log --exec rm",
            "git config user.email attacker@example.com",
            "cargo test && open /tmp/pwned",
        ] {
            assert!(parse(phrase).is_none(), "{phrase}");
            assert!(matches!(parse_outcome(phrase), Some(Parsed::Refused(_))));
        }

        for phrase in [
            "убей процесс",
            "освободи порт",
            "покажи мне штуку",
            "I read about deleting files yesterday",
            "Port 3000 is commonly used in development",
        ] {
            assert!(parse(phrase).is_none(), "{phrase}");
            assert!(parse_outcome(phrase).is_none(), "{phrase}");
        }

        for phrase in [
            "AirDrop only for contacts",
            "эйрдроп only for contacts",
            "Air Drop only for contacts",
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no match for {phrase}"));
            assert_eq!(intent.action, Action::SetAirDropMode, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some("contacts"), "{phrase}");
        }

        for (phrase, expected) in [
            ("free space", Action::ShowDiskUsage),
            ("am I about to run out of storage", Action::ShowDiskUsage),
            ("hoeveel batterij heb ik nog", Action::ShowBattery),
            ("berapa persen baterai", Action::ShowBattery),
            ("what's the time", Action::ShowDate),
            ("what is this machine called", Action::ShowHostname),
            ("how long has this laptop been awake", Action::ShowUptime),
        ] {
            assert_eq!(action(phrase), expected, "{phrase}");
        }
        for phrase in [
            "I feel like using Safari right now",
            "Мне сейчас нужен Safari, покажи его",
            "Je voudrais utiliser Safari maintenant",
        ] {
            match parse_outcome(phrase) {
                Some(Parsed::Intent(intent)) => {
                    assert_eq!(intent.action, Action::OpenApp, "{phrase}")
                }
                other => panic!("no conversational app match for {phrase}: {other:?}"),
            }
        }
    }

    #[test]
    fn http_get_is_typed_and_mutating_methods_are_refused() {
        let url = "https://example.com/search?a=1&b=2";
        let request = format!("сделай curl запрос на {url}");
        let intent = parse(&request).expect("safe GET request should parse");
        assert_eq!(intent.action, Action::FetchUrl);
        assert_eq!(intent.target.as_deref(), Some(url));

        for request in [
            "отправь запрос на google.com",
            "отправь запрос curl на google.com",
            "send a request to example.com/path?q=1",
            "дерни example.dev/api?q=rust&limit=10",
            "example.com에 HTTP 요청 보내기",
        ] {
            let intent = parse(request).expect("scheme-less web request should parse");
            assert_eq!(intent.action, Action::FetchUrl, "{request}");
            assert!(
                intent.target.as_deref().unwrap().starts_with("https://"),
                "{request}"
            );
        }

        for request in [
            "curl -X POST https://example.com",
            "curl --data value https://example.com",
            "curl --header 'X-Test: value' https://example.com",
            "HTTP DELETE request https://example.com/item",
        ] {
            assert!(matches!(parse_outcome(request), Some(Parsed::Refused(_))));
        }
        assert!(matches!(
            parse_outcome("не делай запрос на https://example.com"),
            Some(Parsed::Refused(_))
        ));
        assert!(matches!(
            parse_outcome("do not make a request to example.com"),
            Some(Parsed::Refused(_))
        ));
    }

    #[test]
    fn oversized_input_is_refused_before_any_fallback() {
        let input = "x".repeat(MAX_INPUT_BYTES + 1);
        assert!(parse(&input).is_none());
        assert!(matches!(parse_outcome(&input), Some(Parsed::Refused(_))));
    }

    #[test]
    fn conversational_requests_do_not_require_command_wording() {
        for (phrase, expected) in [
            ("I am done with Safari for now", Action::QuitApp),
            ("I do not want Safari installed anymore", Action::RemoveApp),
            ("Bring README.md up in its usual app", Action::OpenFile),
            ("Where did README.md end up?", Action::FindFile),
            ("I do not need Cargo.lock anymore", Action::DeleteFile),
            (
                "What are the storage hogs in Downloads?",
                Action::FindLargeFiles,
            ),
            (
                "Remind me what folder this terminal is in",
                Action::PrintWorkingDirectory,
            ),
            ("Take this shell over to Downloads", Action::ChangeDirectory),
            ("Which macOS account is this shell using?", Action::WhoAmI),
            ("What kind of Mac am I running?", Action::ShowSystemInfo),
            (
                "What chip and cores does this machine have?",
                Action::ShowCpu,
            ),
            (
                "What address did my Mac get on the local network?",
                Action::ShowNetwork,
            ),
            ("Tell me when it is", Action::ShowDate),
            ("Do I have a node process alive?", Action::FindProcess),
            (
                "Could you cut the Bluetooth radio?",
                Action::SetBluetoothPower,
            ),
            ("كم تبقى من البطارية؟", Action::ShowBattery),
        ] {
            match parse_outcome(phrase) {
                Some(Parsed::Intent(intent)) => assert_eq!(intent.action, expected, "{phrase}"),
                other => panic!("no conversational match for {phrase}: {other:?}"),
            }
        }
    }

    #[test]
    fn negation_and_incidental_ports_do_not_execute() {
        assert!(matches!(
            parse_outcome("Please do not open Safari"),
            Some(Parsed::Refused(_))
        ));
        assert!(matches!(
            parse_outcome("don't enable AirDrop"),
            Some(Parsed::Refused(_))
        ));
        assert!(parse_outcome("I wonder whether port 5173 is a nice number").is_none());
        match parse_outcome("What has grabbed TCP port 5173?") {
            Some(Parsed::Intent(intent)) => assert_eq!(intent.action, Action::FindPortProcess),
            other => panic!("expected port lookup, got {other:?}"),
        }
    }

    #[test]
    fn port_multi_step_is_typed() {
        let steps = parse_multi_step("найди кто сидит на 8765 и выключи его").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].action, Action::FindPortProcess);
        assert_eq!(steps[1].action, Action::KillPortProcess);
        assert_eq!(steps[1].port, Some(8765));
    }

    #[test]
    fn app_multi_step_is_typed() {
        let steps = parse_multi_step("find Chrome then uninstall it").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].action, Action::FindApp);
        assert_eq!(steps[1].action, Action::RemoveApp);
        assert_eq!(steps[1].target.as_deref(), Some("Chrome"));
        let steps = parse_multi_step("открой hosts а потом запусти Chrome").unwrap();
        assert_eq!(steps[0].action, Action::OpenFile);
        assert_eq!(steps[1].action, Action::OpenApp);
        assert_eq!(steps[1].target.as_deref(), Some("Chrome"));
        assert!(parse_multi_step("найди самые большие файлы и удали их").is_none());
    }

    #[test]
    fn coordinated_app_removal_is_typed() {
        let steps = parse_multi_step("удали Roblox и Roblox Studio").unwrap();
        assert_eq!(steps.len(), 2);
        assert!(steps.iter().all(|step| step.action == Action::RemoveApp));
        assert_eq!(steps[0].target.as_deref(), Some("Roblox"));
        assert_eq!(steps[1].target.as_deref(), Some("Roblox Studio"));

        let steps = parse_multi_step("remove VLC and remove Zoom").unwrap();
        assert!(steps.iter().all(|step| step.action == Action::RemoveApp));
        assert_eq!(steps[0].target.as_deref(), Some("VLC"));
        assert_eq!(steps[1].target.as_deref(), Some("Zoom"));

        assert!(parse_multi_step("удали Roblox и $(touch /tmp/pwned)").is_none());
        assert!(parse_multi_step("удали Roblox и очисти кеш npm").is_none());
    }

    #[test]
    fn network_diagnosis_respects_negation() {
        assert!(matches!(
            parse_outcome("не проверяй почему интернет тормозит"),
            Some(Parsed::Refused(_))
        ));
        assert!(matches!(
            parse_outcome("do not diagnose my slow internet"),
            Some(Parsed::Refused(_))
        ));
    }
}

#[cfg(test)]
mod size_generalization_tests {
    use super::*;

    #[test]
    fn size_resolves_arbitrary_directories() {
        let intent = parse("сколько весит src").unwrap();
        assert_eq!(intent.action, Action::DirectorySize);
        assert!(
            intent
                .target
                .as_deref()
                .is_some_and(|target| target.ends_with("/src")),
            "{:?}",
            intent.target
        );
        assert_eq!(
            parse("сколько места занимает terfinder").unwrap().action,
            Action::DirectorySize
        );

        let dir = std::env::temp_dir().join("terfinder-size-test");
        std::fs::create_dir_all(&dir).unwrap();
        let input = format!("сколько весит {}", dir.display());
        let intent = parse(&input).unwrap();
        assert_eq!(
            intent.target.as_deref(),
            Some(dir.display().to_string().as_str())
        );
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(parse("сколько стоит ремонт").map(|i| i.action), None);
    }

    #[test]
    fn loose_size_is_typed() {
        let intent = parse_outcome("найди размер src").unwrap();
        match intent {
            Parsed::Intent(intent) => {
                assert_eq!(intent.action, Action::DirectorySize);
                assert!(
                    intent
                        .target
                        .as_deref()
                        .is_some_and(|target| target.ends_with("/src")),
                    "{:?}",
                    intent.target
                );
            }
            _ => panic!("expected a loose size intent"),
        }
    }
}

#[cfg(test)]
mod open_folder_tests {
    use super::*;

    #[test]
    fn folder_nouns_are_stripped_from_open_targets() {
        for (phrase, action, target) in [
            ("открой папку Centrio", Action::OpenApp, "Centrio"),
            ("открой Centrio папку", Action::OpenApp, "Centrio"),
            ("открой папку Tenvilo", Action::OpenApp, "Tenvilo"),
            ("открой файл hosts", Action::OpenFile, "hosts"),
            ("open folder Downloads", Action::OpenApp, "Downloads"),
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no fast match for {phrase}"));
            assert_eq!(intent.action, action, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some(target), "{phrase}");
        }
        assert!(parse("открой папку").is_none());
    }

    #[test]
    fn enable_verb_reaches_open_branch() {
        let intent = parse("включи VPN").unwrap();
        assert_eq!(intent.action, Action::OpenApp);
        assert_eq!(intent.target.as_deref(), Some("VPN"));
    }

    #[test]
    fn process_list_phrases_are_deterministic() {
        for phrase in [
            "какие щас процессы жрут больше всего",
            "какие процессы запущены",
            "найди работающие процессы",
            "which processes are running",
            "show running processes",
        ] {
            let intent = match parse_outcome(phrase)
                .unwrap_or_else(|| panic!("no fast match for {phrase}"))
            {
                Parsed::Intent(intent) => intent,
                other => panic!("expected intent for {phrase}, got {other:?}"),
            };
            assert_eq!(intent.action, Action::ListProcesses, "{phrase}");
            assert!(intent.target.is_none(), "{phrase}");
        }
        let find = parse("найди процесс python").unwrap();
        assert_eq!(find.action, Action::FindProcess);
        assert_eq!(find.target.as_deref(), Some("python"));

        for (phrase, sort) in [
            ("какой процесс занимает больше всего памяти", "memory"),
            ("что жрет оперативку", "memory"),
            ("покажи процессы по памяти", "memory"),
            ("which process uses the most memory", "memory"),
            ("what is using all my RAM", "memory"),
            ("top memory processes", "memory"),
            ("qué proceso usa más memoria", "memory"),
            ("welcher Prozess braucht am meisten Speicher", "memory"),
            ("quel processus utilise le plus de mémoire", "memory"),
            ("какой процесс грузит процессор", "cpu"),
            ("покажи топ процессов по cpu", "cpu"),
            ("which process is using the CPU", "cpu"),
            ("top CPU processes", "cpu"),
        ] {
            let intent = parse(phrase).unwrap_or_else(|| panic!("no fast match for {phrase}"));
            assert_eq!(intent.action, Action::ListProcesses, "{phrase}");
            assert_eq!(intent.target.as_deref(), Some(sort), "{phrase}");
        }

        assert_eq!(parse("какой процессор").unwrap().action, Action::ShowCpu);
    }
}

#[cfg(test)]
mod multilingual_and_unix_tests {
    use super::*;

    #[test]
    fn unix_commands_and_other_languages() {
        assert_eq!(parse("ls").unwrap().action, Action::ListDirectory);
        assert_eq!(parse("pwd").unwrap().action, Action::PrintWorkingDirectory);
        assert_eq!(parse("cat README.md").unwrap().action, Action::ReadFile);
        assert_eq!(
            parse("mkdir notes").unwrap().action,
            Action::CreateDirectory
        );
        let copy = parse("cp a.txt b.txt").unwrap();
        assert_eq!(copy.action, Action::CopyFile);
        assert_eq!(
            parse("cd Downloads").unwrap().action,
            Action::ChangeDirectory
        );
        assert_eq!(parse("whoami").unwrap().action, Action::WhoAmI);
        assert_eq!(parse("покажи файлы").unwrap().action, Action::ListDirectory);
        assert_eq!(
            parse("list files in Downloads").unwrap().action,
            Action::ListDirectory
        );
        assert_eq!(parse("abre telegram").unwrap().action, Action::OpenApp);
        assert_eq!(parse("öffne safari").unwrap().action, Action::OpenApp);
        assert_eq!(parse("ouvre chrome").unwrap().action, Action::OpenApp);
        assert_eq!(parse("відкрий telegram").unwrap().action, Action::OpenApp);
        assert_eq!(parse("apri safari").unwrap().action, Action::OpenApp);
        assert_eq!(parse("打开 Safari").unwrap().action, Action::OpenApp);
        assert_eq!(
            parse("где я").unwrap().action,
            Action::PrintWorkingDirectory
        );
        assert_eq!(
            parse("where am i").unwrap().action,
            Action::PrintWorkingDirectory
        );
        assert_eq!(
            parse("скопируй a.txt на Desktop").unwrap().action,
            Action::CopyFile
        );
        assert_eq!(
            parse("создай папку notes").unwrap().action,
            Action::CreateDirectory
        );
        assert_eq!(
            parse("перейди в Downloads").unwrap().action,
            Action::ChangeDirectory
        );
        assert_eq!(
            parse("list running processes").unwrap().action,
            Action::ListProcesses
        );
        let config = parse("где конфиг для opencode").unwrap();
        assert_eq!(config.action, Action::FindFile);
        assert_eq!(config.target.as_deref(), Some("opencode"));
        assert_eq!(
            parse("where is the git config").unwrap().target.as_deref(),
            Some("git")
        );
        assert_eq!(
            parse("открой конфиг opencode").unwrap().action,
            Action::OpenFile
        );
        assert_eq!(parse("opencode config").unwrap().action, Action::FindFile);
        match parse_outcome("логи nginx") {
            Some(Parsed::Intent(intent)) => {
                assert_eq!(intent.action, Action::FindFile);
                assert_eq!(intent.target.as_deref(), Some("nginx"));
            }
            other => panic!("expected log lookup, got {other:?}"),
        }
        match parse_outcome("где установлен node") {
            Some(Parsed::Intent(intent)) => {
                assert_eq!(intent.action, Action::FindProcess);
                assert_eq!(intent.target.as_deref(), Some("node"));
            }
            other => panic!("expected binary lookup, got {other:?}"),
        }
        assert_eq!(parse("git status").unwrap().action, Action::RunTool);
        assert_eq!(parse("brew outdated").unwrap().action, Action::RunTool);
        assert!(parse("catalog").is_none() || parse("catalog").unwrap().action != Action::ReadFile);
    }

    #[test]
    fn implied_open_accepts_paths_and_urls() {
        match parse_outcome("https://github.com") {
            Some(Parsed::Intent(intent)) => assert_eq!(intent.action, Action::OpenFile),
            other => panic!("expected open url, got {other:?}"),
        }
        assert_eq!(
            parse_outcome("youtube.com").map(|parsed| match parsed {
                Parsed::Intent(intent) => intent.action,
                _ => Action::Unknown,
            }),
            Some(Action::OpenFile)
        );
    }
}
