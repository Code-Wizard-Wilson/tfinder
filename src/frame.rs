//! Slot parser: goal + kind + name + place, instead of one `if` per sentence.

use crate::intent::{Action, Intent};
use crate::lexicon::{self, Verb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Config,
    Log,
    Process,
    Port,
    Dir,
    Cache,
    Binary,
    File,
}

#[derive(Debug, Default)]
struct Frame {
    goal: Option<Verb>,
    kind: Option<Kind>,
    name: Option<String>,
    location: Option<String>,
    port: Option<u16>,
}

fn token_kind(token: &str) -> Option<Kind> {
    let token = token.to_lowercase();
    const ROWS: &[(Kind, &[&str])] = &[
        (
            Kind::Config,
            &[
                "конфиг",
                "конфіг",
                "config",
                "settings",
                "настройк",
                "налаштуван",
            ],
        ),
        (Kind::Log, &["лог", "log", "logs", "логи", "журна"]),
        (
            Kind::Process,
            &["процесс", "процес", "process", "processes"],
        ),
        (Kind::Port, &["порт", "port", "порту", "порта"]),
        (
            Kind::Dir,
            &["папк", "folder", "directory", "каталог", "dir"],
        ),
        (Kind::Cache, &["кеш", "кэш", "cache", "кэша"]),
        (
            Kind::Binary,
            &[
                "установлен",
                "installed",
                "binary",
                "бинар",
                "executable",
                "бинарь",
                "which",
            ],
        ),
        (Kind::File, &["файл", "file", "files", "файлы", "файла"]),
    ];
    for (kind, markers) in ROWS {
        if markers
            .iter()
            .any(|marker| token == *marker || token.starts_with(marker))
        {
            return Some(*kind);
        }
    }
    None
}

fn drop_token(token: &str) -> bool {
    const DROP: &[&str] = &[
        "для",
        "for",
        "of",
        "the",
        "a",
        "an",
        "в",
        "in",
        "on",
        "на",
        "у",
        "мой",
        "моя",
        "мое",
        "my",
        "me",
        "please",
        "лежит",
        "находится",
        "located",
        "где",
        "де",
        "where",
        "is",
        "are",
        "этот",
        "эта",
        "это",
        "that",
        "this",
        "и",
        "and",
        "или",
        "or",
        "как",
        "how",
        "what",
        "какой",
        "то",
        "что",
        "который",
    ];
    let token = token.to_lowercase();
    DROP.contains(&token.as_str())
        || token_kind(&token).is_some()
        || lexicon::verb(&token).is_some()
}

fn extract(text: &str) -> Frame {
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| ",.!?;:\"'".contains(c))
                .to_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect();
    let mut frame = Frame::default();
    for token in &tokens {
        if frame.goal.is_none()
            && let Some(verb) = lexicon::verb(token)
        {
            frame.goal = Some(verb);
        }
        if frame.kind.is_none()
            && let Some(kind) = token_kind(token)
        {
            frame.kind = Some(kind);
        }
        if frame.port.is_none()
            && let Ok(port) = token.parse::<u16>()
            && port > 0
        {
            frame.port = Some(port);
        }
    }
    frame.location = lexicon::canonical_dir(text).map(str::to_string);
    if let Some(tool) = lexicon::config_tool(text) {
        frame.name = Some(tool);
        if frame.kind.is_none() {
            frame.kind = Some(Kind::Config);
        }
    } else {
        let kept: Vec<&str> = tokens
            .iter()
            .map(String::as_str)
            .filter(|token| {
                !drop_token(token)
                    && frame.location.as_deref().is_none_or(|location| {
                        !token.eq_ignore_ascii_case(location)
                            && !["downloads", "documents", "desktop", "загрузк", "документ"]
                                .iter()
                                .any(|marker| token.starts_with(marker))
                    })
                    && token.parse::<u16>().is_err()
            })
            .collect();
        if !kept.is_empty() && kept.join(" ").len() <= 80 {
            frame.name = Some(kept.join(" "));
        }
    }
    frame
}

/// Conservative compile: only frames with a clear kind or an obvious file name.
pub fn parse(text: &str) -> Option<Intent> {
    let frame = extract(text);
    let lower = text.to_lowercase();
    let port_request = frame.goal.is_some()
        || text.split_whitespace().count() <= 2
        || [
            "who",
            "what",
            "which",
            "кто",
            "что",
            "какой",
            "чем занят",
            "uses",
            "using",
            "listening",
            "occupied",
            "grabbed",
            "слушает",
            "сидит",
            "занимает",
            "держит",
        ]
        .iter()
        .any(|marker| lower.contains(marker));
    if let Some(port) = frame.port
        && ((frame.kind == Some(Kind::Port) && port_request) || frame.goal == Some(Verb::Kill))
    {
        let mut intent = Intent::new(if frame.goal == Some(Verb::Kill) {
            Action::KillPortProcess
        } else {
            Action::FindPortProcess
        });
        intent.port = Some(port);
        return Some(intent);
    }
    if frame.kind == Some(Kind::Config)
        && let Some(name) = frame.name
    {
        return Some(Intent::target(
            match frame.goal {
                Some(Verb::Open) => Action::OpenFile,
                Some(Verb::Read) => Action::ReadFile,
                _ => Action::FindFile,
            },
            name,
        ));
    }
    if frame.kind == Some(Kind::Log)
        && let Some(name) = frame.name
    {
        return Some(Intent::target(Action::FindFile, name));
    }
    if frame.goal == Some(Verb::Clear)
        && frame.kind == Some(Kind::Cache)
        && let Some(name) = frame.name
    {
        return Some(Intent::target(Action::ClearCache, name));
    }
    if (frame.kind == Some(Kind::Binary)
        || (frame.goal == Some(Verb::Find) && frame.kind == Some(Kind::Process)))
        && let Some(name) = frame.name.as_deref()
        && !name.contains('/')
        && !name.contains(' ')
    {
        return Some(Intent::target(Action::FindProcess, name));
    }
    if frame.goal == Some(Verb::List) && matches!(frame.kind, Some(Kind::Dir) | Some(Kind::File)) {
        return Some(if let Some(location) = frame.location {
            Intent::target(Action::ListDirectory, location)
        } else if let Some(name) = frame.name {
            Intent::target(Action::ListDirectory, name)
        } else {
            Intent::new(Action::ListDirectory)
        });
    }
    if frame.goal == Some(Verb::Size) {
        let target = frame.location.or(frame.name)?;
        crate::system::existing_dir(&target)?;
        return Some(Intent::target(Action::DirectorySize, target));
    }
    if frame.goal == Some(Verb::Read)
        && let Some(name) = frame.name
    {
        return Some(Intent::target(Action::ReadFile, name));
    }
    if frame.goal == Some(Verb::Cd) {
        return Some(Intent::target(
            Action::ChangeDirectory,
            frame.location.or(frame.name).unwrap_or_else(|| "~".into()),
        ));
    }
    if matches!(frame.goal, Some(Verb::Find) | Some(Verb::Open))
        && let Some(name) = frame.name
        && (name.contains('.') || name.contains('/') || name.starts_with('~'))
    {
        return Some(Intent::target(
            if frame.goal == Some(Verb::Open) {
                Action::OpenFile
            } else {
                Action::FindFile
            },
            name,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_cover_classes_not_sentences() {
        let log = parse("логи nginx").unwrap();
        assert_eq!(log.action, Action::FindFile);
        assert_eq!(log.target.as_deref(), Some("nginx"));
        let bin = parse("где установлен node").unwrap();
        assert_eq!(bin.action, Action::FindProcess);
        assert_eq!(bin.target.as_deref(), Some("node"));
        let conf = parse("settings for ghostty").unwrap();
        assert_eq!(conf.action, Action::FindFile);
        assert_eq!(conf.target.as_deref(), Some("ghostty"));
        assert!(parse("сколько стоит ремонт").is_none());
        assert!(parse("hello world").is_none());
    }
}
