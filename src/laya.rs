use crate::config::Config;
use crate::intent::{Action, CAPABILITIES, FileType, Intent, SemanticGoal, UnsupportedGoal};
use crate::lexicon::{self, Verb};
use serde_json::json;
use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn socket_path() -> Result<PathBuf, String> {
    let home = crate::system::home().map_err(|e| e.to_string())?;
    Ok(home.join(".cache/terfinder/laya.sock"))
}

fn daemon_log(cache_dir: &std::path::Path) -> PathBuf {
    cache_dir.join("daemon.log")
}

fn managed_python() -> Option<PathBuf> {
    let path = crate::system::home()
        .ok()?
        .join(".local/share/terfinder/venv/bin/python");
    path.is_file().then_some(path)
}

pub fn python_path(config: &Config) -> Option<PathBuf> {
    env::var_os("TERFINDER_PYTHON")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| config.python.clone().filter(|path| path.is_file()))
        .or_else(managed_python)
        .or_else(|| crate::system::executable("python3"))
}

/// Last characters of the daemon log, so a startup failure explains itself.
fn log_tail(path: &std::path::Path) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let tail: String = text.chars().rev().take(1200).collect();
    tail.chars().rev().collect()
}

fn connect(start: bool, config: &Config) -> Result<UnixStream, String> {
    let path = socket_path()?;
    if let Ok(stream) = UnixStream::connect(&path) {
        return Ok(stream);
    }
    if !start {
        return Err("daemon is not running".into());
    }
    let python = python_path(config).unwrap_or_else(|| PathBuf::from("python3"));
    let cache_dir = path.parent().ok_or("invalid socket path")?;
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let log_path = daemon_log(cache_dir);
    let mut command = Command::new(&python);
    command
        .arg("-c")
        .arg(include_str!("../daemon/server.py"))
        .arg(&path)
        .stdin(Stdio::null());
    if let Ok(log) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        && let Ok(err) = log.try_clone()
    {
        command.stdout(Stdio::from(log));
        command.stderr(Stdio::from(err));
    } else {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }
    if env::var_os("TERFINDER_IDLE_TIMEOUT").is_none() {
        command.env(
            "TERFINDER_IDLE_TIMEOUT",
            config.idle_timeout_seconds.to_string(),
        );
    }
    if env::var_os("TERFINDER_MODEL").is_none() {
        command.env("TERFINDER_MODEL", config.checkpoint());
    }
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn setsid() -> i32;
            }
            if setsid() < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start local Laya daemon: {e}"))?;
    for _ in 0..100 {
        if let Ok(stream) = UnixStream::connect(&path) {
            return Ok(stream);
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let tail = log_tail(&log_path);
            return Err(format!(
                "Laya daemon exited immediately ({status}). Full output: {}\n{tail}Install it with `pip install laya-mlx`, or point TERFINDER_PYTHON at an environment that has it.",
                log_path.display()
            ));
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let tail = log_tail(&log_path);
    Err(format!(
        "Laya daemon did not become ready in 5s. Full output: {}\n{tail}Configure TERFINDER_PYTHON with a Python environment containing laya-mlx.",
        log_path.display()
    ))
}

fn request(
    value: serde_json::Value,
    start: bool,
    config: &Config,
) -> Result<serde_json::Value, String> {
    let mut stream = connect(start, config)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(180)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    writeln!(stream, "{value}").map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let response: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    if let Some(error) = response.get("error").and_then(|x| x.as_str()) {
        return Err(format!("Laya daemon: {error}"));
    }
    Ok(response)
}

pub fn status() -> Result<serde_json::Value, String> {
    request(json!({"control":"status"}), false, &Config::default())
}
pub fn start(config: &Config) -> Result<serde_json::Value, String> {
    let _ = connect(true, config)?;
    status()
}
pub fn stop() -> Result<serde_json::Value, String> {
    request(json!({"control":"stop"}), false, &Config::default())
}

fn extract_port(text: &str) -> Option<u16> {
    text.split(|c: char| !c.is_ascii_digit())
        .find_map(|part| part.parse::<u16>().ok().filter(|p| *p > 0))
}

fn extract_app(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut names = Vec::new();
    for path in crate::system::applications().ok()? {
        let name = path.file_stem()?.to_string_lossy().to_string();
        if lower.contains(&name.to_lowercase()) {
            names.push(name);
        }
    }
    names.sort_by_key(|s| std::cmp::Reverse(s.len()));
    names.first().cloned()
}

fn extract_named_directory(text: &str) -> Option<String> {
    let names = ["Downloads", "Documents", "Desktop"];
    let words = text.split(|c: char| !c.is_alphanumeric());
    for word in words {
        if let Some(name) = names.iter().find(|name| word.eq_ignore_ascii_case(name)) {
            return Some((*name).to_string());
        }
    }
    None
}

/// Pull out a path-like object without asking the model to reproduce user
/// text. Only a quoted value, a recognizable path/file token, or a known home
/// directory is accepted. The boolean reports whether the resolved object is
/// a directory.
fn extract_resource(text: &str) -> Option<(String, bool)> {
    let mut candidates = Vec::new();
    for quote in ['"', '\''] {
        let mut parts = text.split(quote);
        while let (Some(_), Some(value)) = (parts.next(), parts.next()) {
            let value = value.trim();
            if !value.is_empty() {
                candidates.push(value.to_string());
            }
        }
    }
    for raw in text.split_whitespace() {
        let value = raw.trim_matches(|c: char| "!?;,:([]){}`\"'".contains(c));
        if value.is_empty() {
            continue;
        }
        if value.contains(['/', '~', '.'])
            || value.eq_ignore_ascii_case("hosts")
            || ["Desktop", "Documents", "Downloads"]
                .iter()
                .any(|name| value.eq_ignore_ascii_case(name))
        {
            candidates.push(value.to_string());
        }
    }
    candidates.sort_by_key(|value| std::cmp::Reverse(value.len()));
    candidates.dedup();
    for candidate in candidates {
        if let Some(path) = crate::system::existing_dir(&candidate) {
            return Some((path.display().to_string(), true));
        }
        if let Ok(paths) = crate::system::find_files(&candidate)
            && paths.len() == 1
        {
            let path = &paths[0];
            return Some((path.display().to_string(), path.is_dir()));
        }
        // A filename can legitimately be the object of FIND_FILE even when it
        // is not in the fast bounded locations. Keep it typed as a file name;
        // later lookup remains bounded and read-only.
        if candidate.contains('.') && !candidate.starts_with('.') {
            return Some((candidate, false));
        }
    }
    None
}

fn directory_query(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "больш",
        "large",
        "largest",
        "size",
        "весит",
        "сколько",
        "how large",
        "занимает",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// A deterministic size phrasing (`сколько весит X`, `size X`, `занимает
/// место X`): the goal is unambiguous even when the model abstains. `стоит`
/// (costs money) is deliberately absent.
fn size_led_query(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "вес",
        "размер",
        "size",
        "объем",
        "объём",
        "weight",
        "занима",
        "место",
        "du",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// True when the request talks about a port, so capability filtering may pick
/// the PORT domain. A bare number inside a word (`Qwerty123`) is not a port.
fn port_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "порт",
        "порту",
        "порта",
        "port",
        "listening",
        "listen",
        "слушает",
        "сидит",
        "занимает",
        "держит",
        "occupied",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || (extract_port(text).is_some()
            && (process_context(text)
                || ["кто", "что", "who", "what", "whos", "whose", "чей"]
                    .iter()
                    .any(|word| lower.contains(word))))
}

/// The query explicitly talks about running processes (but not `процессор` /
/// `processor`): pre-filter the capabilities so the domain question is skipped.
fn process_context(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .any(|word| {
            let word = word.to_lowercase();
            (word.starts_with("процесс") && !word.starts_with("процессор"))
                || word.starts_with("проццес")
                || (word.starts_with("process") && !word.starts_with("processor"))
        })
}

fn control_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "bluetooth",
        "блютуз",
        "блютус",
        "airdrop",
        "air drop",
        "эйрдроп",
        "эирдроп",
        "stage manager",
        "stagemanager",
        "стейдж менеджер",
        "стейджменеджер",
        "постановщик",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn control_target(text: &str, action: Action) -> Option<String> {
    let lower = text.to_lowercase();
    if action == Action::SetAirDropMode {
        if [
            "только контакт",
            "только для контакт",
            "contacts only",
            "contact only",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            return Some("contacts".into());
        }
        if ["для всех", "everyone", "everybody", "all people"]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            return Some("everyone".into());
        }
    }
    if [
        "turn off",
        "disable",
        "deactivate",
        "shut off",
        "отключ",
        "выключ",
        "деактив",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || (lower
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .any(|word| word == "off")
            && ["turn ", "switch ", "set "]
                .iter()
                .any(|verb| lower.starts_with(verb)))
    {
        Some("off".into())
    } else if [
        "turn on",
        "enable",
        "activate",
        "bring back",
        "включ",
        "активир",
        "верни",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || (lower
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .any(|word| word == "on")
            && ["turn ", "switch ", "set "]
                .iter()
                .any(|verb| lower.starts_with(verb)))
    {
        Some("on".into())
    } else {
        None
    }
}

fn http_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find_map(crate::lexicon::normalize_http_target)
}

fn http_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    if http_url(text).is_none() {
        return false;
    }
    let explicit = ["curl ", "fetch ", "request ", "get ", "http get "]
        .iter()
        .any(|marker| lower.starts_with(marker));
    let noun = [
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
    let verb = [
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
    explicit || (noun && verb)
}

/// Words that never name a directory: size verbs, file/time quantifiers, and
/// filler. Short entries match whole tokens only, longer ones are prefixes.
const SIZE_DROP: &[&str] = &[
    "сколько",
    "вес",
    "размер",
    "объем",
    "место",
    "места",
    "занима",
    "стоит",
    "size",
    "du",
    "weight",
    "найди",
    "найти",
    "find",
    "show",
    "покажи",
    "показать",
    "посмотри",
    "у",
    "мне",
    "меня",
    "мой",
    "моя",
    "мое",
    "это",
    "этот",
    "the",
    "my",
    "this",
    "that",
    "please",
    "пожалуйста",
    "и",
    "а",
    "файл",
    "files",
    "file",
    "видео",
    "video",
    "pdf",
    "фото",
    "photo",
    "image",
    "picture",
    "картин",
    "документ",
    "папк",
    "folder",
    "dir",
    "director",
    "больш",
    "large",
    "big",
    "largest",
    "самы",
    "тяжел",
    "heavy",
    "в",
    "in",
    "из",
    "of",
    "на",
    "on",
    "за",
    "last",
    "days",
    "day",
    "день",
    "дней",
    "week",
    "month",
    "year",
    "месяц",
    "недел",
    "год",
    "гб",
    "gb",
    "мб",
    "mb",
    "кб",
    "kb",
];

fn drop_size_word(token: &str) -> bool {
    if token.contains(['/', '~']) {
        return false;
    }
    if token.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    matches!(lexicon::verb(token), Some(Verb::Size) | Some(Verb::Find))
        || SIZE_DROP.iter().any(|word| {
            if word.len() <= 3 {
                token == *word
            } else {
                token.starts_with(word)
            }
        })
}

/// The object of a size or large-file request, stripped of verbs, quantifiers,
/// and filler: `найди большие файлы в src` → `src`.
fn size_candidate(text: &str) -> Option<String> {
    if !text.contains(['/', '~'])
        && let Some(dir) = extract_named_directory(text)
    {
        return Some(dir);
    }
    let kept: Vec<&str> = text
        .split_whitespace()
        .map(|raw| raw.trim_matches(|c: char| "!?,.;:()\"'".contains(c)))
        .filter(|token| !token.is_empty() && !drop_size_word(&token.to_lowercase()))
        .collect();
    if kept.is_empty() || kept.join(" ").len() > 200 {
        return None;
    }
    Some(kept.join(" "))
}

/// A plausible directory reference: it exists, it is a path, or it is a single
/// name. Multi-word phrasing that resolves nowhere is left to the model.
fn plausible_directory(candidate: &str) -> bool {
    crate::system::existing_dir(candidate).is_some()
        || candidate.contains('/')
        || candidate.starts_with('~')
        || !candidate.contains(' ')
}

fn fill_directory_constraints(text: &str, intent: &mut Intent) -> Result<(), String> {
    let lower = text.to_lowercase();
    if intent.action != Action::FindLargeFiles {
        return Ok(());
    }
    intent.minimum_size_bytes = Some(0);
    if lower.contains("видео") || lower.contains("video") {
        intent.file_type = Some(FileType::Video);
    } else if lower.contains("pdf") {
        intent.file_type = Some(FileType::Pdf);
    } else if lower.contains("image") || lower.contains("фото") {
        intent.file_type = Some(FileType::Image);
    }
    if lower.contains("за месяц") || lower.contains("last 30 days") {
        intent.max_age_days = Some(30);
    } else if ["за неделю", "last week", "last 7 days"]
        .iter()
        .any(|v| lower.contains(v))
    {
        intent.max_age_days = Some(7);
    } else if ["за год", "last year", "last 365 days"]
        .iter()
        .any(|v| lower.contains(v))
    {
        intent.max_age_days = Some(365);
    }
    if (lower.contains("за ") || lower.contains("last ")) && intent.max_age_days.is_none() {
        return Err("Time filter is not yet supported; no search was run.".into());
    }
    Ok(())
}

pub enum Classified {
    Executable(Intent),
    Unsupported(SemanticGoal),
}

fn abstain(text: &str) -> String {
    crate::alias::record_miss(text);
    if text
        .chars()
        .any(|character| ('\u{0400}'..='\u{04ff}').contains(&character))
    {
        format!(
            "Пока не смог надёжно понять: {text}\nНичего не выполнено. Формулировка сохранена для улучшения распознавания."
        )
    } else {
        format!(
            "Not confidently understood: {text}\nNothing was executed. The wording was saved to improve recognition."
        )
    }
}

pub fn classify(text: &str, start_daemon: bool, config: &Config) -> Result<Classified, String> {
    let _ = connect(start_daemon, config)?;
    let daemon = status()?;
    let expected_model = env::var("TERFINDER_MODEL").unwrap_or_else(|_| config.checkpoint().into());
    if daemon.get("model").and_then(|value| value.as_str()) != Some(expected_model.as_str()) {
        return Err("Running daemon uses a different checkpoint; stop it with `tf daemon stop` and retry. No changes made.".into());
    }
    let app_target = extract_app(text);
    let port_target = extract_port(text);
    let resource_target = extract_resource(text);
    let file_target = resource_target
        .as_ref()
        .filter(|(_, is_directory)| !is_directory)
        .map(|(target, _)| target.clone());
    let directory_target = resource_target
        .as_ref()
        .filter(|(_, is_directory)| *is_directory)
        .map(|(target, _)| target.clone())
        .or_else(|| extract_named_directory(text));
    let size_target = size_candidate(text).filter(|candidate| plausible_directory(candidate));
    let selected_domain = if control_context(text) {
        Some("CONTROL")
    } else if http_context(text) {
        Some("WEB")
    } else if app_target.is_some() {
        Some("APP")
    } else if port_target.is_some() && port_context(text) {
        Some("PORT")
    } else if process_context(text) {
        Some("PROCESS")
    } else if file_target.is_some() {
        Some("FILE")
    } else if directory_target.is_some() || (directory_query(text) && size_target.is_some()) {
        Some("DIRECTORY")
    } else {
        None
    };
    let capabilities: Vec<_> = CAPABILITIES
        .iter()
        .filter(|c| match selected_domain {
            Some("APP") => matches!(
                c.action,
                Action::FindApp
                    | Action::RemoveApp
                    | Action::RemoveAppCompletely
                    | Action::OpenApp
                    | Action::QuitApp
            ),
            Some("PORT") => c.name.contains("PORT"),
            Some("PROCESS") => matches!(
                c.action,
                Action::FindProcess | Action::KillProcess | Action::ListProcesses
            ),
            Some("CONTROL") => matches!(
                c.action,
                Action::SetBluetoothPower
                    | Action::SetAirDropMode
                    | Action::SetStageManager
            ),
            Some("WEB") => c.action == Action::FetchUrl,
            Some("FILE") => matches!(
                c.action,
                Action::FindFile
                    | Action::OpenFile
                    | Action::MoveFile
                    | Action::RenameFile
                    | Action::DeleteFile
                    | Action::ReadFile
                    | Action::CopyFile
            ),
            Some("DIRECTORY") => matches!(
                c.action,
                Action::DeleteDirectory
                    | Action::DirectorySize
                    | Action::FindLargeFiles
                    | Action::ListDirectory
                    | Action::CreateDirectory
                    | Action::PrintWorkingDirectory
                    | Action::ChangeDirectory
            ),
            _ => true,
        })
        .map(|c| json!({"name":c.name,"description":c.description,"required":c.required,"examples":c.examples}))
        .collect();
    let debug = env::var_os("TERFINDER_DEBUG").is_some();
    let mut raw = request(
        json!({"text":text,"capabilities":capabilities,"debug":debug,"threshold":config.confidence_threshold}),
        start_daemon,
        config,
    )?;
    if debug && let Some(diagnostics) = raw.get("diagnostics") {
        eprintln!("Laya decisions: {diagnostics}");
    }
    if let Some(object) = raw.as_object_mut() {
        object.remove("diagnostics");
    }
    if raw.get("action").and_then(|x| x.as_str()) == Some("UNCLEAR") {
        return Err(abstain(text));
    }
    if raw.get("action").and_then(|x| x.as_str()) == Some("UNSUPPORTED") {
        if let Some(goal) = raw.get("semantic_goal")
            && let Ok(semantic_goal) = serde_json::from_value::<UnsupportedGoal>(goal.clone())
        {
            let confidence = raw
                .get("semantic_confidence")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0) as f32;
            if confidence.is_finite() && confidence >= config.confidence_threshold.max(0.85) {
                let target = match semantic_goal {
                    UnsupportedGoal::SetDefaultBrowser
                    | UnsupportedGoal::EnableLoginItem
                    | UnsupportedGoal::DisableLoginItem => app_target,
                    _ => None,
                };
                return Ok(Classified::Unsupported(SemanticGoal {
                    action: "UNSUPPORTED",
                    semantic_goal,
                    target,
                    confidence,
                }));
            }
        }
        if app_target.is_none()
            && size_led_query(text)
            && let Some(target) = size_target.as_deref()
            && crate::system::existing_dir(target).is_none()
        {
            return Err(format!("No such directory: {target}. No changes made."));
        }
        return Err(abstain(text));
    }
    let mut intent =
        crate::intent::validate_model_json(&raw.to_string(), config.confidence_threshold)
            .ok_or_else(|| {
                "Malformed or low-confidence Laya answer; no changes made.".to_string()
            })?;
    let cap = CAPABILITIES
        .iter()
        .find(|c| c.action == intent.action)
        .ok_or("Laya returned an unknown action; no changes made.")?;
    match cap.action {
        Action::FindPortProcess | Action::KillPortProcess => intent.port = port_target,
        Action::FindApp
        | Action::RemoveApp
        | Action::RemoveAppCompletely
        | Action::OpenApp
        | Action::QuitApp => intent.target = app_target,
        Action::SetBluetoothPower | Action::SetAirDropMode | Action::SetStageManager => {
            intent.target = control_target(text, intent.action)
        }
        Action::FetchUrl => intent.target = http_url(text),
        Action::DirectorySize | Action::FindLargeFiles => {
            intent.target = directory_target
                .as_ref()
                .or(size_target.as_ref())
                .map(|target| {
                    crate::system::existing_dir(target)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| target.clone())
                });
            fill_directory_constraints(text, &mut intent)?;
        }
        Action::FindFile | Action::OpenFile | Action::DeleteFile => {
            intent.target = file_target;
        }
        Action::ListProcesses
        | Action::ShowDiskUsage
        | Action::ShowBattery
        | Action::PrintWorkingDirectory
        | Action::ShowDate
        | Action::WhoAmI
        | Action::ShowHostname
        | Action::ShowSystemInfo
        | Action::ShowUptime
        | Action::ShowMemory
        | Action::ShowCpu
        | Action::ShowNetwork
        | Action::DiagnoseNetwork
        | Action::RunTool => {}
        Action::ReadFile => {
            intent.target = file_target;
        }
        Action::ListDirectory | Action::CreateDirectory | Action::ChangeDirectory => {
            if intent.target.is_none() {
                intent.target = directory_target
                    .as_ref()
                    .or(size_target.as_ref())
                    .map(|target| {
                        crate::system::existing_dir(target)
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| target.clone())
                    });
            }
        }
        Action::CopyFile => {}
        _ => {
            return Err(format!(
                "Understood {} but deterministic entity extraction for this phrasing is unavailable. No changes made.",
                cap.name
            ));
        }
    }
    let needs_entity = matches!(
        cap.action,
        Action::FindPortProcess
            | Action::KillPortProcess
            | Action::FindApp
            | Action::RemoveApp
            | Action::RemoveAppCompletely
            | Action::OpenApp
            | Action::QuitApp
            | Action::DirectorySize
            | Action::FindLargeFiles
            | Action::FindFile
            | Action::OpenFile
            | Action::MoveFile
            | Action::RenameFile
            | Action::DeleteFile
            | Action::DeleteDirectory
            | Action::ClearCache
            | Action::SetBluetoothPower
            | Action::SetAirDropMode
            | Action::SetStageManager
            | Action::FetchUrl
            | Action::ReadFile
            | Action::CreateDirectory
            | Action::ChangeDirectory
            | Action::CopyFile
    );
    if matches!(intent.action, Action::CopyFile)
        && (intent.source.as_deref().is_none_or(str::is_empty)
            || intent.destination.as_deref().is_none_or(str::is_empty))
    {
        return Err(format!(
            "Understood {} but could not resolve a target from the request. No changes made.",
            cap.name
        ));
    }
    if needs_entity && intent.port.is_none() && intent.target.is_none() && intent.source.is_none() {
        return Err(format!(
            "Understood {} but could not resolve a target from the request. No changes made.",
            cap.name
        ));
    }
    if matches!(
        intent.action,
        Action::DirectorySize | Action::FindLargeFiles
    ) && let Some(target) = intent.target.as_deref()
        && crate::system::existing_dir(target).is_none()
    {
        return Err(format!("No such directory: {target}. No changes made."));
    }
    Ok(Classified::Executable(intent))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_port_without_shell() {
        assert_eq!(extract_port("what's on 8765; rm -rf /"), Some(8765));
    }

    #[test]
    fn extracts_existing_file_from_free_form_request() {
        let result = extract_resource("I need to see what Cargo.toml says").unwrap();
        assert!(result.0.ends_with("Cargo.toml"));
        assert!(!result.1);
    }

    #[test]
    fn directory_constraints_are_typed() {
        assert_eq!(
            extract_named_directory("в Downloads"),
            Some("Downloads".into())
        );
        assert_eq!(extract_named_directory("DownloadsExtra"), None);
        let mut intent = Intent::target(Action::FindLargeFiles, "Downloads");
        fill_directory_constraints("найди большие видео за месяц в Downloads", &mut intent)
            .unwrap();
        assert_eq!(intent.file_type, Some(FileType::Video));
        assert_eq!(intent.max_age_days, Some(30));
        assert_eq!(intent.minimum_size_bytes, Some(0));
    }
}

#[cfg(test)]
mod size_generalization_tests {
    use super::*;

    #[test]
    fn size_candidates_drop_filler() {
        assert_eq!(size_candidate("сколько весит Anton"), Some("Anton".into()));
        assert_eq!(
            size_candidate("найди большие файлы в src"),
            Some("src".into())
        );
        assert_eq!(size_candidate("и сколько весит?"), None);
        assert_eq!(size_candidate("в Downloads"), Some("Downloads".into()));
        assert!(size_candidate("сколько весит Documents/src").is_some());
    }

    #[test]
    fn port_context_requires_port_words() {
        assert!(port_context("что занимает 8765"));
        assert!(port_context("кто на 8765"));
        assert!(!port_context("сколько весит Qwerty123"));
        assert!(!port_context("сколько весит Anton"));
    }

    #[test]
    fn plausible_directory_prefers_existing_paths() {
        assert!(plausible_directory("src"));
        assert!(plausible_directory("Qwerty123"));
        assert!(plausible_directory("/no/such/path"));
        assert!(!plausible_directory("стоит ремонт"));
    }
}

#[cfg(test)]
mod process_domain_tests {
    use super::*;

    #[test]
    fn process_context_matches_processes_not_processors() {
        assert!(process_context("какие щас процессы жрут больше всего"));
        assert!(process_context("покажи проццесы"));
        assert!(process_context("list running processes"));
        assert!(!process_context("сколько весит процессор"));
        assert!(!process_context("сколько весит Downloads"));
    }

    #[test]
    fn port_context_with_process_words_and_port_numbers() {
        assert!(port_context("процесс на 8765"));
        assert!(!port_context("найди процесс python"));
    }
}
