//! Deterministic linguistic layer: verbs, politeness fillers, aliases, typo
//! tolerance, unsupported goals and typed file filters.

use crate::intent::FileType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Open,
    Delete,
    Find,
    Move,
    Rename,
    Clear,
    Kill,
    Size,
    List,
    Copy,
    Read,
    Create,
    Cd,
    Pwd,
}

const STEMS: &[(Verb, &[&str])] = &[
    (
        Verb::Open,
        &[
            "откр",
            "запусти",
            "запуст",
            "запуск",
            "включ",
            "відкри",
            "увімкн",
            "open",
            "launch",
            "start",
            "run",
            "enable",
            "abre",
            "abrir",
            "lanza",
            "inicia",
            "öffn",
            "start",
            "ouvr",
            "lance",
            "démarr",
            "apri",
            "avvia",
            "otwórz",
            "otworz",
            "uruchom",
            "aç",
            "başlat",
        ],
    ),
    (
        Verb::Delete,
        &[
            "удал",
            "снес",
            "убер",
            "выкин",
            "выброс",
            "сотри",
            "избав",
            "видал",
            "прибер",
            "remove",
            "uninstall",
            "delete",
            "erase",
            "borra",
            "elimin",
            "lösch",
            "entfern",
            "supprim",
            "efface",
            "apaga",
            "exclui",
            "cancella",
            "usuń",
            "usun",
        ],
    ),
    (
        Verb::Find,
        &[
            "найд",
            "найти",
            "поищ",
            "ищи",
            "где",
            "де",
            "покаж",
            "показ",
            "знайд",
            "покаж",
            "find",
            "locate",
            "search",
            "show",
            "where",
            "busca",
            "encontr",
            "muestra",
            "dónde",
            "donde",
            "finde",
            "such",
            "zeige",
            "trouv",
            "cherch",
            "montre",
            "procura",
            "mostra",
            "trova",
            "cerca",
            "znajd",
            "pokaż",
            "pokaz",
        ],
    ),
    (
        Verb::Move,
        &[
            "перемест",
            "переміс",
            "move",
            "mueve",
            "mover",
            "verschieb",
            "déplac",
            "deplac",
            "sposta",
            "przenieś",
            "przenies",
        ],
    ),
    (
        Verb::Rename,
        &[
            "переимен",
            "переймен",
            "rename",
            "renombr",
            "benenn",
            "renomm",
            "rinomin",
        ],
    ),
    (
        Verb::Clear,
        &[
            "очист",
            "почист",
            "clean",
            "clear",
            "limpia",
            "limpiar",
            "leere",
            "vide",
            "pulisci",
        ],
    ),
    (
        Verb::Kill,
        &[
            "убей",
            "убь",
            "уби",
            "грохн",
            "выключ",
            "заверш",
            "останов",
            "освобод",
            "закрой",
            "закры",
            "закрий",
            "закри",
            "вимкн",
            "kill",
            "stop",
            "free",
            "terminate",
            "quit",
            "close",
            "cierra",
            "cerrar",
            "mata",
            "schließ",
            "schliess",
            "beend",
            "ferm",
            "arrêt",
            "arret",
            "fecha",
            "chiudi",
            "zatrzymaj",
        ],
    ),
    (
        Verb::Size,
        &[
            "сколько",
            "размер",
            "весит",
            "занима",
            "розмір",
            "size",
            "weight",
            "объем",
            "объём",
            "tamaño",
            "tamano",
            "größe",
            "grosse",
            "taille",
            "tamanho",
            "dimension",
            "rozmiar",
        ],
    ),
    (
        Verb::List,
        &["список", "перечисл", "list", "lista", "liste", "elenca"],
    ),
    (
        Verb::Copy,
        &[
            "скопир",
            "копир",
            "копію",
            "copy",
            "copia",
            "copiar",
            "kopier",
            "copie",
        ],
    ),
    (Verb::Read, &["прочит", "содержим", "read", "leer", "lire"]),
    (
        Verb::Create,
        &[
            "создай",
            "создать",
            "созда",
            "створи",
            "mkdir",
            "create",
            "crea",
            "erstell",
        ],
    ),
    (
        Verb::Cd,
        &[
            "перейд",
            "зайд",
            "перейти",
            "chdir",
            "goto",
            "entra",
            "entrar",
            "gehe",
        ],
    ),
];

const FORMS: &[(Verb, &[&str])] = &[
    (
        Verb::Open,
        &[
            "открой",
            "открыть",
            "запусти",
            "запустить",
            "відкрий",
            "відкрити",
            "open",
            "launch",
            "start",
            "run",
            "abre",
            "abrir",
            "öffne",
            "öffnen",
            "ouvre",
            "ouvrir",
            "apri",
            "otwórz",
        ],
    ),
    (
        Verb::Delete,
        &[
            "удали",
            "удалить",
            "снеси",
            "снести",
            "убери",
            "убрать",
            "выкинь",
            "видали",
            "видалити",
            "remove",
            "uninstall",
            "delete",
            "borra",
            "borrar",
            "lösche",
            "supprime",
            "apaga",
            "elimina",
        ],
    ),
    (
        Verb::Find,
        &[
            "найди",
            "найти",
            "покажи",
            "показать",
            "поищи",
            "знайди",
            "find",
            "locate",
            "show",
            "where",
            "busca",
            "finde",
            "trouve",
            "mostra",
            "trova",
        ],
    ),
    (
        Verb::Move,
        &["перемести", "переместить", "move", "mueve", "déplace"],
    ),
    (Verb::Rename, &["переименуй", "переименовать", "rename"]),
    (
        Verb::Clear,
        &["очисти", "очистить", "почисти", "clean", "clear"],
    ),
    (
        Verb::Kill,
        &[
            "убей",
            "убить",
            "выключи",
            "выключить",
            "заверши",
            "останови",
            "освободи",
            "закрой",
            "закрыть",
            "закрий",
            "kill",
            "stop",
            "free",
            "quit",
            "close",
            "terminate",
            "cierra",
            "schließe",
            "ferme",
            "chiudi",
        ],
    ),
    (
        Verb::Size,
        &["сколько", "размер", "весит", "size", "weight", "объем"],
    ),
    (Verb::List, &["list", "lista", "liste", "покажи"]),
    (Verb::Copy, &["скопируй", "скопировать", "copy", "copia"]),
    (Verb::Read, &["прочитай", "read", "cat"]),
    (
        Verb::Create,
        &["создай", "создать", "mkdir", "create", "crea"],
    ),
    (Verb::Cd, &["перейди", "перейти", "cd", "entra"]),
];

/// Exact first-token unix and short words. These must not be stem-matched
/// (`cat` must not swallow `catalog`, `dir` must not swallow `directory`).
const COMMANDS: &[(&str, Verb)] = &[
    ("ls", Verb::List),
    ("dir", Verb::List),
    ("cp", Verb::Copy),
    ("rm", Verb::Delete),
    ("mv", Verb::Move),
    ("cd", Verb::Cd),
    ("chdir", Verb::Cd),
    ("cat", Verb::Read),
    ("head", Verb::Read),
    ("type", Verb::Read),
    ("pwd", Verb::Pwd),
    ("mkdir", Verb::Create),
    ("md", Verb::Create),
    ("du", Verb::Size),
    ("df", Verb::Find),
    ("ps", Verb::Find),
    ("which", Verb::Find),
    ("whereis", Verb::Find),
    ("whoami", Verb::Pwd),
    ("hostname", Verb::Pwd),
    ("date", Verb::Pwd),
    ("lsof", Verb::Find),
];

/// Phrase-initial verbs without spaces (CJK) or long foreign openers, rewritten
/// to an English verb so the rest of the parser can run.
const LEADING: &[(&str, &str)] = &[
    ("打开", "open"),
    ("开启", "open"),
    ("启动", "open"),
    ("运行", "open"),
    ("删除", "delete"),
    ("卸载", "delete"),
    ("去掉", "delete"),
    ("找到", "find"),
    ("查找", "find"),
    ("搜索", "find"),
    ("显示", "show"),
    ("列出", "list"),
    ("移动", "move"),
    ("重命名", "rename"),
    ("清理", "clear"),
    ("清除", "clear"),
    ("关闭", "close"),
    ("退出", "quit"),
    ("杀掉", "kill"),
    ("结束", "kill"),
    ("复制", "copy"),
    ("拷贝", "copy"),
    ("创建文件夹", "mkdir"),
    ("新建文件夹", "mkdir"),
    ("进入", "cd"),
    ("转到", "cd"),
    ("查看", "cat"),
    ("当前目录", "pwd"),
    ("開いて", "open"),
    ("起動", "open"),
    ("削除", "delete"),
    ("探して", "find"),
    ("見つけて", "find"),
    ("閉じて", "close"),
    ("終了", "quit"),
    ("コピー", "copy"),
    ("열어", "open"),
    ("실행", "open"),
    ("삭제", "delete"),
    ("찾아", "find"),
    ("보여", "show"),
    ("종료", "quit"),
    ("닫아", "close"),
];

/// Word pairs that are dropped before phrase matching: politeness and filler.
const FILLERS: &[&str] = &[
    "i",
    "me",
    "my",
    "the",
    "a",
    "an",
    "this",
    "that",
    "just",
    "please",
    "pls",
    "plz",
    "kindly",
    "could",
    "would",
    "can",
    "you",
    "por",
    "favor",
    "bitte",
    "s'il",
    "vous",
    "plaît",
    "plait",
    "per",
    "favore",
    "proszę",
    "proszę",
    "будь",
    "ласка",
    "some",
    "really",
    "actually",
    "maybe",
    "perhaps",
    "let",
    "us",
    "go",
    "ahead",
    "now",
    "also",
    "very",
    "мне",
    "мой",
    "моя",
    "мое",
    "мои",
    "этот",
    "эта",
    "эти",
    "просто",
    "наверное",
    "конечно",
    "давай",
    "уже",
    "вот",
    "ну",
    "бы",
    "ли",
    "дай",
    "скажи",
    "вообще",
    "чуть",
    "немного",
];

/// Non-verb prefixes removed before parsing. Stripping only happens when a
/// verb follows, so the command itself is never eaten.
const PREFIXES: &[&str] = &[
    "i would like to ",
    "i'd like to ",
    "i want to ",
    "i need to ",
    "i wanna ",
    "can you ",
    "could you ",
    "would you ",
    "can i ",
    "may i ",
    "please ",
    "please, ",
    "kindly ",
    "just ",
    "help me ",
    "let me ",
    "let's ",
    "go ahead and ",
    "pls ",
    "plz ",
    "now ",
    "also ",
    "por favor ",
    "bitte ",
    "s'il te plaît ",
    "s'il vous plaît ",
    "per favore ",
    "proszę ",
    "будь ласка ",
    "quiero ",
    "ich möchte ",
    "ich will ",
    "je veux ",
    "eu quero ",
    "voglio ",
    "хочу ",
    "я хочу ",
    "请 ",
    "хочется ",
    "мне нужно ",
    "мне надо ",
    "мне бы ",
    "нужно ",
    "надо ",
    "можно ",
    "а можно ",
    "пожалуйста ",
    "пожалуйста, ",
    "плиз ",
    "давай ",
    "конечно ",
    "просто ",
    "уже ",
    "скажи ",
    "ну ",
    "а ",
    "и ",
    "вот ",
];

const DIR_WORDS: &[(&str, &str)] = &[
    ("downloads", "Downloads"),
    ("download", "Downloads"),
    ("загрузки", "Downloads"),
    ("загрузок", "Downloads"),
    ("загрузках", "Downloads"),
    ("загрузке", "Downloads"),
    ("докачки", "Downloads"),
    ("докачек", "Downloads"),
    ("documents", "Documents"),
    ("document", "Documents"),
    ("документы", "Documents"),
    ("документов", "Documents"),
    ("desktop", "Desktop"),
    ("десктоп", "Desktop"),
    ("рабочийстол", "Desktop"),
    ("рабочийстола", "Desktop"),
    ("descargas", "Downloads"),
    ("descarga", "Downloads"),
    ("téléchargements", "Downloads"),
    ("telechargements", "Downloads"),
    ("downloadsordner", "Downloads"),
    ("heruntergeladen", "Downloads"),
    ("завантаження", "Downloads"),
    ("завантажень", "Downloads"),
    ("pobrane", "Downloads"),
    ("documentos", "Documents"),
    ("documente", "Documents"),
    ("dokumente", "Documents"),
    ("documenti", "Documents"),
    ("документи", "Documents"),
    ("документів", "Documents"),
    ("dokumenty", "Documents"),
    ("escritorio", "Desktop"),
    ("bureau", "Desktop"),
    ("schreibtisch", "Desktop"),
    ("pulpit", "Desktop"),
    ("стільниця", "Desktop"),
    ("стільниці", "Desktop"),
];

/// Spoken app names mapped onto installed bundle names.
const APP_WORDS: &[(&str, &str)] = &[
    ("телеграм", "telegram"),
    ("телеграмм", "telegram"),
    ("хром", "chrome"),
    ("гугл", "google"),
    ("дискорд", "discord"),
    ("сафари", "safari"),
    ("калькулятор", "calculator"),
    ("калькульятор", "calculator"),
    ("терминал", "terminal"),
    ("настройки", "system settings"),
    ("настройка", "system settings"),
    ("параметры", "system settings"),
    ("системные настройки", "system settings"),
    ("файндер", "finder"),
    ("файндера", "finder"),
    ("почта", "mail"),
    ("почты", "mail"),
    ("заметки", "notes"),
    ("заметок", "notes"),
    ("сообщения", "messages"),
    ("календарь", "calendar"),
    ("календаря", "calendar"),
    ("фото", "photos"),
    ("музыка", "music"),
    ("музыки", "music"),
    ("подкасты", "podcasts"),
    ("предпросмотр", "preview"),
    ("превью", "preview"),
    ("текстовый редактор", "textedit"),
    ("монитор активности", "activity monitor"),
    ("активность", "activity monitor"),
    ("спотлайт", "spotlight"),
    ("visual studio code", "visual studio code"),
    ("вс код", "visual studio code"),
    ("vscode", "visual studio code"),
    ("vs code", "visual studio code"),
    ("браузер", "browser"),
    ("ватсап", "whatsapp"),
    ("вотсап", "whatsapp"),
    ("спотифай", "spotify"),
    ("слак", "slack"),
    ("зум", "zoom"),
    ("зуум", "zoom"),
    ("фигма", "figma"),
    ("обсидиан", "obsidian"),
    ("фаерфокс", "firefox"),
    ("файрфокс", "firefox"),
    ("конфигурация", "system settings"),
    ("configuración", "system settings"),
    ("configuracion", "system settings"),
    ("ajustes", "system settings"),
    ("einstellungen", "system settings"),
    ("réglages", "system settings"),
    ("reglages", "system settings"),
    ("paramètres", "system settings"),
    ("parametres", "system settings"),
    ("impostazioni", "system settings"),
    ("налаштування", "system settings"),
    ("ustawienia", "system settings"),
    ("whatsapp", "whatsapp"),
    ("spotify", "spotify"),
    ("slack", "slack"),
    ("zoom", "zoom"),
    ("figma", "figma"),
    ("obsidian", "obsidian"),
    ("firefox", "firefox"),
];

const APP_PHRASES: &[(&str, &str)] = &[
    ("системные настройки", "system settings"),
    ("рабочий стол", "desktop"),
    ("visual studio code", "visual studio code"),
    ("vs code", "visual studio code"),
    ("текстовый редактор", "textedit"),
    ("монитор активности", "activity monitor"),
];

const TIME_UNITS: &[(&str, u32)] = &[
    ("день", 1),
    ("дня", 1),
    ("дней", 1),
    ("дн", 1),
    ("day", 1),
    ("days", 1),
    ("неделю", 7),
    ("недели", 7),
    ("неделя", 7),
    ("недель", 7),
    ("week", 7),
    ("weeks", 7),
    ("месяц", 30),
    ("месяца", 30),
    ("месяцев", 30),
    ("месяцeв", 30),
    ("month", 30),
    ("months", 30),
    ("год", 365),
    ("года", 365),
    ("лет", 365),
    ("year", 365),
    ("years", 365),
];

const UNKNOWN_TIME_WORDS: &[&str] = &[
    "час",
    "часа",
    "часов",
    "hour",
    "hours",
    "минут",
    "минута",
    "минуты",
    "minute",
    "minutes",
    "секунд",
    "second",
    "seconds",
    "вчера",
    "yesterday",
    "сегодня",
    "today",
    "недавно",
    "recently",
];

const SIZE_UNITS: &[(&str, u64)] = &[
    ("кб", 1_024),
    ("килобайт", 1_024),
    ("килобайта", 1_024),
    ("килобайтов", 1_024),
    ("kb", 1_024),
    ("mb", 1_048_576),
    ("мб", 1_048_576),
    ("мегабайт", 1_048_576),
    ("мегабайта", 1_048_576),
    ("мегабайтов", 1_048_576),
    ("gb", 1_073_741_824),
    ("гб", 1_073_741_824),
    ("гигабайт", 1_073_741_824),
    ("гигабайта", 1_073_741_824),
    ("гигабайтов", 1_073_741_824),
    ("gib", 1_073_741_824),
    ("mib", 1_048_576),
    ("tb", 1_099_511_627_776),
    ("тб", 1_099_511_627_776),
];

const COMMON_TLDS: &[&str] = &[
    "app", "ai", "blog", "cloud", "club", "co", "com", "dev", "edu", "fun", "gov", "group", "info",
    "io", "live", "me", "mil", "net", "news", "online", "org", "page", "pro", "ru", "site", "shop",
    "store", "studio", "tech", "travel", "tv", "su", "xyz", "zone",
];

/// Damerau-style single edit (substitution, adjacent swap, insertion).
pub fn edit1(left: &str, right: &str) -> bool {
    let a: Vec<_> = left.chars().collect();
    let b: Vec<_> = right.chars().collect();
    if a == b {
        return true;
    }
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    if a.len() == b.len() {
        let differing: Vec<_> = a
            .iter()
            .zip(&b)
            .enumerate()
            .filter_map(|(i, (x, y))| (x != y).then_some(i))
            .collect();
        return differing.len() == 1
            || (differing.len() == 2
                && differing[1] == differing[0] + 1
                && a[differing[0]] == b[differing[1]]
                && a[differing[1]] == b[differing[0]]);
    }
    let (short, long) = if a.len() < b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    let mut i = 0;
    let mut j = 0;
    let mut skipped = false;
    while i < short.len() && j < long.len() {
        if short[i] == long[j] {
            i += 1;
            j += 1;
        } else if skipped {
            return false;
        } else {
            skipped = true;
            j += 1;
        }
    }
    true
}

pub fn rewrite_leading(input: &str) -> String {
    let trimmed = input.trim();
    let mut best: Option<(&str, &str)> = None;
    for (needle, verb) in LEADING {
        if starts_with_ignore_case(trimmed, needle)
            && best.is_none_or(|(current, _)| needle.chars().count() > current.chars().count())
        {
            best = Some((needle, verb));
        }
    }
    let Some((needle, verb)) = best else {
        return trimmed.to_string();
    };
    let rest = strip_char_prefix(trimmed, needle.chars().count()).trim();
    if rest.is_empty() {
        verb.to_string()
    } else {
        format!("{verb} {rest}")
    }
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    let mut text = text.chars();
    let mut prefix = prefix.chars();
    loop {
        match (text.next(), prefix.next()) {
            (_, None) => return true,
            (None, Some(_)) => return false,
            (Some(left), Some(right)) => {
                if !left.to_lowercase().eq(right.to_lowercase()) {
                    return false;
                }
            }
        }
    }
}

fn token_matches_stem(token: &str, stem: &str) -> bool {
    if token == stem {
        return true;
    }
    if stem.len() < 3 || !token.starts_with(stem) {
        return false;
    }
    if stem.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return matches!(
            &token[stem.len()..],
            "s" | "es" | "ed" | "d" | "ing" | "er" | "ers" | "en" | "n" | "ning" | "e"
        );
    }
    true
}

fn strip_char_prefix(text: &str, chars: usize) -> &str {
    text.char_indices()
        .nth(chars)
        .map(|(index, _)| &text[index..])
        .unwrap_or("")
}

/// Resolve a single token to a canonical action verb, tolerating inflection
/// and one typo.
pub fn verb(token: &str) -> Option<Verb> {
    let token = token.trim().to_lowercase();
    if token.is_empty() {
        return None;
    }
    for (command, verb) in COMMANDS {
        if token == *command {
            return Some(*verb);
        }
    }
    for (verb, stems) in STEMS {
        for stem in *stems {
            if token_matches_stem(&token, stem) {
                return Some(*verb);
            }
        }
    }
    if token.chars().count() >= 4 {
        for (verb, forms) in FORMS {
            for form in *forms {
                if edit1(&token, form) {
                    return Some(*verb);
                }
            }
        }
    }
    None
}

/// Lowercase, drop punctuation and politeness filler. Used for `contains`
/// matching only; positional parsing always uses the untouched text.
pub fn normalize_text(input: &str) -> String {
    let lowered = input.to_lowercase();
    let cleaned: String = lowered
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect();
    cleaned
        .split_whitespace()
        .filter(|token| !FILLERS.contains(token))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove politeness prefixes such as `I would like to` or `пожалуйста`.
/// A prefix is only removed when a verb follows it.
pub fn strip_prefixes(input: &str) -> String {
    let mut text = input.trim().to_string();
    for _ in 0..4 {
        let mut stripped = false;
        for prefix in PREFIXES {
            let Some(rest) = strip_once(&text, prefix) else {
                continue;
            };
            if rest.trim().is_empty() {
                continue;
            }
            if starts_with_verb(rest) || rest.to_lowercase().starts_with("get rid of ") {
                text = rest.trim_start().to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    text
}

fn strip_once<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if !text.to_lowercase().starts_with(prefix) {
        return None;
    }
    let count = prefix.chars().count();
    text.char_indices()
        .nth(count)
        .map(|(index, _)| &text[index..])
}

fn starts_with_verb(text: &str) -> bool {
    text.split_whitespace()
        .next()
        .and_then(|word| verb(word.trim_matches(|c: char| !c.is_alphanumeric())))
        .is_some()
}

/// Leading filler words of an object phrase (`у меня chrome` → `chrome`).
pub fn strip_object_filler(text: &str) -> String {
    const LEADING: &[&str] = &[
        "у меня ",
        "у меня",
        "мне ",
        "мой ",
        "моя ",
        "мое ",
        "this ",
        "the ",
        "my ",
        "me ",
        "a ",
        "an ",
        "some ",
        "its ",
        "его ",
        "её ",
        "это ",
        "этот ",
    ];
    let mut out = text.trim().to_string();
    for _ in 0..3 {
        let lower = out.to_lowercase();
        let Some(next) = LEADING
            .iter()
            .find_map(|prefix| lower.starts_with(prefix).then(|| &out[prefix.len()..]))
        else {
            break;
        };
        if next.trim().is_empty() {
            break;
        }
        out = next.trim_start().to_string();
    }
    // Object nouns that name nothing by themselves: `открой папку Tenvilo` →
    // `Tenvilo`, `открой Centrio папку` → `Centrio`.
    const NOUNS: &[&str] = &[
        "папку",
        "папка",
        "папки",
        "папке",
        "папкой",
        "папок",
        "папочку",
        "папочки",
        "каталог",
        "каталога",
        "каталоги",
        "каталоге",
        "директорию",
        "директория",
        "директории",
        "folder",
        "folders",
        "dir",
        "dirs",
        "directory",
        "directories",
        "file",
        "files",
        "файл",
        "файла",
        "файлы",
        "файлу",
    ];
    loop {
        let words: Vec<&str> = out.split_whitespace().collect();
        if words.len() <= 1 {
            if words.len() == 1 && NOUNS.contains(&words[0].to_lowercase().as_str()) {
                return String::new();
            }
            break;
        }
        if NOUNS.contains(&words[0].to_lowercase().as_str()) {
            out = words[1..].join(" ");
        } else if NOUNS.contains(&words[words.len() - 1].to_lowercase().as_str()) {
            out = words[..words.len() - 1].join(" ");
        } else {
            break;
        }
    }
    out
}

/// True when an object phrase names no concrete target (`снеси приложение`,
/// `грохни всё подряд`). Such requests must fall back to the model so it can
/// ask what the user meant instead of guessing.
pub fn is_vague_object(text: &str) -> bool {
    const VAGUE: &[&str] = &[
        "app",
        "application",
        "this",
        "that",
        "it",
        "something",
        "anything",
        "everything",
        "whatever",
        "any",
        "файл",
        "файлы",
        "файлами",
        "file",
        "files",
        "приложение",
        "приложения",
        "программа",
        "программу",
        "процесс",
        "процессы",
        "process",
        "processes",
        "порт",
        "порту",
        "port",
        "штука",
        "штуку",
        "thing",
        "это",
        "эту",
        "этот",
        "то",
        "что-нибудь",
        "что нибудь",
        "что-то",
        "что-либо",
        "всё",
        "все",
        "всю",
        "всех",
        "подряд",
        "что угодно",
        "любой",
        "любое",
        "любую",
    ];
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return true;
    }
    let words = normalized
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    !words.is_empty() && words.iter().all(|word| VAGUE.contains(word))
}

/// `весит`, `занимает места`, `how large` → the request is about a size.
/// Markers are matched on whole words so `перемести` never reads as `мест`.
pub fn has_size_marker(text: &str) -> bool {
    const STEMS: &[&str] = &[
        "весит",
        "размер",
        "занима",
        "объем",
        "обём",
        "сколько",
        "величин",
    ];
    const EXACT: &[&str] = &[
        "мест",
        "место",
        "места",
        "вес",
        "space",
        "weight",
        "how",
        "размеры",
    ];
    let lower = text.to_lowercase();
    lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .any(|word| STEMS.iter().any(|stem| word.starts_with(stem)) || EXACT.contains(&word))
}

/// `приложение Figma`, `app Safari` → `Figma`, `Safari`.
pub fn strip_app_prefix(text: &str) -> String {
    const PREFIXES: &[&str] = &[
        "приложение ",
        "приложения ",
        "приложеньку ",
        "программа ",
        "программу ",
        "app ",
        "application ",
    ];
    for prefix in PREFIXES {
        if text
            .get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        {
            return text[prefix.len()..].trim_start().to_string();
        }
    }
    text.trim().to_string()
}

/// Locate a well-known home directory anywhere in the request.
pub fn canonical_dir(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    if lower.contains("рабочий стол") || lower.contains("рабочего стола") {
        return Some("Desktop");
    }
    for word in lower.split(|c: char| !c.is_alphanumeric()) {
        if [
            "стол",
            "стола",
            "столу",
            "столе",
            "столом",
            "столы",
            "столах",
        ]
        .contains(&word)
        {
            return Some("Desktop");
        }
        if let Some(dir) = DIR_WORDS.iter().find(|(key, _)| *key == word) {
            return Some(dir.1);
        }
    }
    None
}

/// Rewrite spoken app words so `открой телеграм` resolves to Telegram.app.
pub fn canonical_app_query(query: &str) -> String {
    let lower = query.to_lowercase();
    for (phrase, alias) in APP_PHRASES {
        if lower.contains(phrase) {
            return (*alias).to_string();
        }
    }
    lower
        .split_whitespace()
        .map(|word| {
            let bare = word.trim_matches(|c: char| !c.is_alphanumeric());
            APP_WORDS
                .iter()
                .find(|(key, _)| *key == bare)
                .map(|(_, alias)| (*alias).to_string())
                .unwrap_or_else(|| bare.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn file_type(text: &str) -> Option<FileType> {
    const VIDEO: &[&str] = &["видео", "video", "ролик", "клип", "movie", "фильм"];
    const PDF: &[&str] = &["pdf"];
    const IMAGE: &[&str] = &[
        "фото",
        "фотограф",
        "image",
        "picture",
        "картин",
        "снимк",
        "изображ",
    ];
    let lower = text.to_lowercase();
    if VIDEO.iter().any(|marker| lower.contains(marker)) {
        Some(FileType::Video)
    } else if PDF.iter().any(|marker| lower.contains(marker)) {
        Some(FileType::Pdf)
    } else if IMAGE.iter().any(|marker| lower.contains(marker)) {
        Some(FileType::Image)
    } else {
        None
    }
}

/// `за месяц`, `last 30 days`, `за 3 дня` → a day count.
/// Unknown time windows return an error instead of silently dropping the filter.
pub fn age_days(text: &str) -> Result<Option<u32>, String> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    for (index, word) in words.iter().enumerate() {
        if let Some((_, days)) = TIME_UNITS.iter().find(|(unit, _)| *unit == *word) {
            let scale = words[..index]
                .iter()
                .rev()
                .find_map(|previous| previous.parse::<u32>().ok());
            return Ok(Some(days.saturating_mul(scale.unwrap_or(1))));
        }
        if UNKNOWN_TIME_WORDS.contains(word) {
            return Err("That time window is not supported; no search was run.".into());
        }
    }
    if ["last ", "прошл", "последн"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        if lower.contains("месяц") || lower.contains("month") {
            return Ok(Some(30));
        }
        if lower.contains("недел") || lower.contains("week") {
            return Ok(Some(7));
        }
        if lower.contains("год") || lower.contains("year") {
            return Ok(Some(365));
        }
        if lower.contains("дн") || lower.contains("day") {
            return Ok(Some(1));
        }
        return Err("That time window is not supported; no search was run.".into());
    }
    Ok(None)
}

/// `больше 1 гб`, `over 500 mb` → a byte threshold.
pub fn min_size(text: &str) -> Option<u64> {
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_alphanumeric() && !c.is_ascii_digit())
        .filter(|w| !w.is_empty())
        .collect();
    let multiplier = |unit: &str| {
        SIZE_UNITS
            .iter()
            .find(|(name, _)| *name == unit)
            .map(|(_, value)| *value)
    };
    for word in &words {
        if let Some((number, unit)) = split_number_unit(word)
            && let Some(scale) = multiplier(unit)
        {
            return number.parse::<u64>().ok()?.checked_mul(scale);
        }
    }
    for pair in words.windows(2) {
        if let (Ok(number), Some(scale)) = (pair[0].parse::<u64>(), multiplier(pair[1])) {
            return number.checked_mul(scale);
        }
    }
    None
}

fn split_number_unit(word: &str) -> Option<(&str, &str)> {
    let digits = word
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(index, _)| index)
        .last()?;
    let (number, unit) = word.split_at(digits + 1);
    if unit.is_empty() {
        return None;
    }
    Some((number, unit))
}

/// `где конфиг для opencode`, `where is the git config` → `opencode`, `git`.
/// Any tool name works; this is not a per-app table.
pub fn config_tool(text: &str) -> Option<String> {
    const MARKERS: &[&str] = &[
        "конфиг",
        "конфіг",
        "конфигурац",
        "конфігурац",
        "config",
        "settings",
        "настройк",
        "налаштуван",
        "параметр",
        "prefs",
        "preference",
        "dotfile",
    ];
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.')
        .filter(|word| !word.is_empty())
        .collect();
    if !words
        .iter()
        .any(|word| MARKERS.iter().any(|marker| word.starts_with(marker)))
    {
        return None;
    }
    const DROP: &[&str] = &[
        "где",
        "де",
        "where",
        "is",
        "are",
        "the",
        "a",
        "an",
        "for",
        "of",
        "to",
        "in",
        "on",
        "my",
        "me",
        "please",
        "для",
        "для",
        "у",
        "мне",
        "мой",
        "моя",
        "мое",
        "файл",
        "file",
        "files",
        "путь",
        "path",
        "location",
        "лежит",
        "находится",
        "находится",
        "find",
        "найди",
        "найти",
        "покажи",
        "показать",
        "show",
        "open",
        "открой",
        "открыть",
        "read",
        "cat",
        "прочит",
        "which",
        "what",
        "какой",
        "какая",
        "какие",
        "dónde",
        "donde",
        "está",
        "esta",
        "el",
        "la",
        "los",
        "wo",
        "ist",
        "die",
        "der",
        "das",
        "où",
        "est",
        "le",
        "du",
        "de",
        "des",
        "o",
        "do",
        "da",
    ];
    let kept: Vec<&str> = words
        .iter()
        .copied()
        .filter(|word| {
            !MARKERS.iter().any(|marker| word.starts_with(marker)) && !DROP.contains(word)
        })
        .collect();
    match kept.as_slice() {
        [] => None,
        [name] if name.chars().count() >= 2 => Some((*name).to_string()),
        names if names.iter().all(|name| name.chars().count() >= 2) && names.len() <= 4 => {
            Some(names.join(" "))
        }
        _ => None,
    }
}

pub fn looks_like_url(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.") {
        return true;
    }
    if lower.contains(' ') || lower.contains('/') {
        return false;
    }
    let mut parts = lower.split('.');
    let host = parts.next().unwrap_or("");
    let tld = match (parts.next(), parts.next()) {
        (Some(tld), None) => tld,
        _ => return false,
    };
    !host.is_empty()
        && !host.starts_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && COMMON_TLDS.contains(&tld)
}

/// Normalize one URL-like token for a safe HTTP request. Public DNS names
/// default to HTTPS; loopback hosts default to HTTP for local development.
pub fn normalize_http_target(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| "\"'(){}<>,".contains(c))
        .trim_end_matches(['.', ';']);
    let value = ["에서", "으로", "에", "に"]
        .iter()
        .find_map(|suffix| trimmed.strip_suffix(suffix))
        .unwrap_or(trimmed);
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = value.to_lowercase();
    let (explicit_scheme, rest) = if lower.starts_with("https://") {
        (Some("https"), &value[8..])
    } else if lower.starts_with("http://") {
        (Some("http"), &value[7..])
    } else {
        (None, value)
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        let host = &bracketed[..end];
        let suffix = &bracketed[end + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':')?)
        };
        (host, port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if host.contains(':') {
            return None;
        }
        (host, Some(port))
    } else {
        (authority, None)
    };
    if let Some(port) = port
        && (port.is_empty() || port.parse::<u16>().ok().filter(|port| *port > 0).is_none())
    {
        return None;
    }
    let ip = host.parse::<std::net::IpAddr>().ok();
    let localhost = host.eq_ignore_ascii_case("localhost") || ip.is_some_and(|ip| ip.is_loopback());
    let labels = host.split('.').collect::<Vec<_>>();
    let valid_dns = labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
        && labels.last().is_some_and(|tld| {
            (tld.len() >= 2 && tld.chars().all(|character| character.is_ascii_alphabetic()))
                || tld.starts_with("xn--")
        });
    if !localhost && ip.is_none() && !valid_dns {
        return None;
    }
    let scheme = explicit_scheme.unwrap_or(if localhost { "http" } else { "https" });
    Some(format!("{scheme}://{rest}"))
}

/// Goals TerFinder understands but deliberately does not execute.
pub fn unsupported_goal(text: &str) -> Option<crate::intent::UnsupportedGoal> {
    use crate::intent::UnsupportedGoal;
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has_word = |candidates: &[&str]| words.iter().any(|word| candidates.contains(word));

    if lower.contains("default browser")
        || lower.contains("браузер по умолчанию")
        || lower.contains("браузером по умолчанию")
        || (lower.contains("по умолчанию")
            && has_word(&[
                "браузер",
                "browser",
                "chrome",
                "firefox",
                "safari",
                "edge",
                "opera",
            ]))
    {
        return Some(UnsupportedGoal::SetDefaultBrowser);
    }
    if [
        "at login",
        "on login",
        "login item",
        "при входе",
        "при загрузке",
        "при старте",
        "автозагрузк",
        "автозапуск",
        "загружаться",
        "запускаться при",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        if has_word(&[
            "запрети",
            "запретить",
            "отключи",
            "отключить",
            "disable",
            "stop",
            "prevent",
        ]) || lower.contains("не запуска")
            || lower.contains("не должна")
            || lower.contains("don't")
            || lower.contains("don t")
            || lower.contains("dont")
            || lower.contains("do not")
        {
            return Some(UnsupportedGoal::DisableLoginItem);
        }
        if has_word(&[
            "enable",
            "allow",
            "включи",
            "включить",
            "разреши",
            "запуска",
            "запусти",
        ]) || lower.contains("запуска")
        {
            return Some(UnsupportedGoal::EnableLoginItem);
        }
        return None;
    }
    if has_word(&["update", "обнови", "обновить", "обновление", "обновления"])
        && (words.iter().any(|word| {
            ["macos", "mac", "os", "ос", "мак"].contains(word)
                || word.starts_with("систем")
                || word.starts_with("обновл")
        }) || lower.contains("macos"))
    {
        return Some(UnsupportedGoal::UpdateOs);
    }
    if has_word(&[
        "download",
        "скачай",
        "скачать",
        "скачал",
        "загрузи",
        "загрузить",
    ]) {
        return Some(UnsupportedGoal::DownloadFile);
    }
    if has_word(&[
        "install",
        "поставь",
        "поставить",
        "установи",
        "установить",
        "установка",
        "инсталлируй",
    ]) && !has_word(&["uninstall", "деинстал"])
    {
        return Some(UnsupportedGoal::InstallApp);
    }
    if has_word(&[
        "send",
        "отправь",
        "отправить",
        "напиши",
        "написать",
        "write",
    ]) && has_word(&["email", "e-mail", "письмо", "письма", "мейл", "mail"])
    {
        return Some(UnsupportedGoal::SendEmail);
    }
    if has_word(&[
        "create",
        "создай",
        "создать",
        "зарегистрируй",
        "новый",
        "новая",
        "register",
    ]) && has_word(&[
        "account",
        "аккаунт",
        "аккаунта",
        "учетную",
        "учётную",
        "регистрация",
    ]) {
        return Some(UnsupportedGoal::CreateAccount);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::UnsupportedGoal;

    #[test]
    fn verb_covers_inflections_and_typos() {
        assert_eq!(verb("ls"), Some(Verb::List));
        assert_eq!(verb("cat"), Some(Verb::Read));
        assert_eq!(verb("catalog"), None);
        assert_eq!(verb("abre"), Some(Verb::Open));
        assert_eq!(verb("öffne"), Some(Verb::Open));
        assert_eq!(verb("ouvre"), Some(Verb::Open));
        assert_eq!(verb("відкрий"), Some(Verb::Open));
        assert_eq!(verb("открой"), Some(Verb::Open));
        assert_eq!(verb("открыть"), Some(Verb::Open));
        assert_eq!(verb("запусти"), Some(Verb::Open));
        assert_eq!(verb("запускай"), Some(Verb::Open));
        assert_eq!(verb("запусти"), Some(Verb::Open));
        assert_eq!(verb("remvoe"), Some(Verb::Delete));
        assert_eq!(verb("clera"), Some(Verb::Clear));
        assert_eq!(verb("сниси"), Some(Verb::Delete));
        assert_eq!(verb("удли"), Some(Verb::Delete));
        assert_eq!(verb("закрой"), Some(Verb::Kill));
        assert_eq!(verb("quit"), Some(Verb::Kill));
        assert_eq!(verb("chrome"), None);
        assert_eq!(verb("opencode"), None);
        assert_eq!(verb("opening"), Some(Verb::Open));
        assert_eq!(verb("downloads"), None);
        assert_eq!(rewrite_leading("打开 Safari"), "open Safari");
        assert_eq!(rewrite_leading("削除 Chrome"), "delete Chrome");
    }

    #[test]
    fn prefixes_only_strip_before_a_verb() {
        assert_eq!(
            strip_prefixes("I would like to remove Chrome"),
            "remove Chrome"
        );
        assert_eq!(
            strip_prefixes("пожалуйста открой telegram"),
            "открой telegram"
        );
        assert_eq!(strip_prefixes("а найди chrome"), "найди chrome");
        assert_eq!(strip_prefixes("chrome"), "chrome");
        assert_eq!(strip_prefixes("пожалуйста"), "пожалуйста");
    }

    #[test]
    fn aliases_resolve_spoken_names() {
        assert_eq!(canonical_app_query("Телеграм"), "telegram");
        assert_eq!(canonical_app_query("открой"), "открой");
        assert_eq!(canonical_app_query("Google Хром"), "google chrome");
        assert_eq!(canonical_dir("в Downloads"), Some("Downloads"));
        assert_eq!(canonical_dir("докачки"), Some("Downloads"));
        assert_eq!(canonical_dir("на рабочем столе"), Some("Desktop"));
        assert_eq!(canonical_dir("report.pdf"), None);
        assert_eq!(canonical_dir("столбик.txt"), None);
        assert_eq!(strip_app_prefix("приложение Figma"), "Figma");
        assert_eq!(strip_app_prefix("app Safari"), "Safari");
        assert_eq!(strip_app_prefix("Figma"), "Figma");
        assert!(is_vague_object("приложение"));
        assert!(is_vague_object("всё подряд"));
        assert!(!is_vague_object("chrome"));
    }

    #[test]
    fn typed_filters_are_extracted() {
        assert_eq!(age_days("за месяц").unwrap(), Some(30));
        assert_eq!(age_days("за 3 дня").unwrap(), Some(3));
        assert_eq!(age_days("last week").unwrap(), Some(7));
        assert!(age_days("за 2 часа").is_err());
        assert_eq!(min_size("больше 1 гб"), Some(1_073_741_824));
        assert_eq!(min_size("over 500 mb"), Some(500 * 1_048_576));
        assert_eq!(file_type("найди большие видео"), Some(FileType::Video));
        assert_eq!(file_type("скачай pdf"), Some(FileType::Pdf));
    }

    #[test]
    fn unsupported_goals_are_recognized() {
        assert_eq!(
            unsupported_goal("change default browser to google chrome"),
            Some(UnsupportedGoal::SetDefaultBrowser)
        );
        assert_eq!(
            unsupported_goal("enable google chrome at login"),
            Some(UnsupportedGoal::EnableLoginItem)
        );
        assert_eq!(
            unsupported_goal("запрети discord запускаться при входе"),
            Some(UnsupportedGoal::DisableLoginItem)
        );
        assert_eq!(
            unsupported_goal("поставь мне docker"),
            Some(UnsupportedGoal::InstallApp)
        );
        assert_eq!(
            unsupported_goal("uninstall chrome"),
            None,
            "uninstall must not look like install"
        );
        assert_eq!(
            unsupported_goal("обнови macos"),
            Some(UnsupportedGoal::UpdateOs)
        );
        assert_eq!(
            unsupported_goal("send an email to alex"),
            Some(UnsupportedGoal::SendEmail)
        );
        assert_eq!(
            unsupported_goal("создай новый аккаунт"),
            Some(UnsupportedGoal::CreateAccount)
        );
        assert_eq!(unsupported_goal("удали chrome"), None);
        assert_eq!(unsupported_goal("открой telegram"), None);
    }

    #[test]
    fn url_detection_ignores_file_names() {
        assert!(looks_like_url("youtube.com"));
        assert!(looks_like_url("https://github.com"));
        assert!(looks_like_url("www.google.com"));
        assert!(!looks_like_url("report.pdf"));
        assert!(!looks_like_url("node_modules"));
        assert_eq!(
            config_tool("где конфиг для opencode").as_deref(),
            Some("opencode")
        );
        assert_eq!(
            config_tool("where is the git config").as_deref(),
            Some("git")
        );
        assert_eq!(config_tool("opencode config").as_deref(), Some("opencode"));
        assert_eq!(config_tool("открой настройки").as_deref(), None);
        assert_eq!(config_tool("где python").as_deref(), None);
    }

    #[test]
    fn normalizes_http_targets_without_a_tld_allowlist() {
        assert_eq!(
            normalize_http_target("google.de/search?q=rust").as_deref(),
            Some("https://google.de/search?q=rust")
        );
        assert_eq!(
            normalize_http_target("HTTPS://Example.COM/path").as_deref(),
            Some("https://Example.COM/path")
        );
        assert_eq!(
            normalize_http_target("localhost:8080/health").as_deref(),
            Some("http://localhost:8080/health")
        );
        assert_eq!(
            normalize_http_target("127.0.0.1:3000/api").as_deref(),
            Some("http://127.0.0.1:3000/api")
        );
        assert!(normalize_http_target("file:///etc/passwd").is_none());
        assert!(normalize_http_target("https://user:secret@example.com").is_none());
        assert!(normalize_http_target("not-a-domain").is_none());
        assert!(normalize_http_target("https://example.com:99999").is_none());
    }
}

#[cfg(test)]
mod object_filler_tests {
    use super::strip_object_filler;

    #[test]
    fn object_nouns_are_stripped_from_both_ends() {
        assert_eq!(strip_object_filler("папку Tenvilo"), "Tenvilo");
        assert_eq!(strip_object_filler("Centrio папку"), "Centrio");
        assert_eq!(strip_object_filler("у меня папку Tenvilo"), "Tenvilo");
        assert_eq!(strip_object_filler("file отчет.pdf"), "отчет.pdf");
        assert_eq!(strip_object_filler("папку"), "");
        assert_eq!(strip_object_filler("chrome"), "chrome");
    }
}

#[cfg(test)]
mod enable_verb_tests {
    use super::*;

    #[test]
    fn enable_is_open_not_kill() {
        assert_eq!(verb("включи"), Some(Verb::Open));
        assert_eq!(verb("включить"), Some(Verb::Open));
        assert_eq!(verb("включ"), Some(Verb::Open));
        assert_eq!(verb("enable"), Some(Verb::Open));
        assert_eq!(verb("выключить"), Some(Verb::Kill));
        assert_eq!(verb("выключи"), Some(Verb::Kill));
    }
}
