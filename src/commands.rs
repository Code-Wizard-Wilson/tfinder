//! Argv-style commands (`ls`, `pwd`, `cat`, `mkdir`) mapped onto typed intents.
//! Flags are whitelisted; nothing is passed to a shell.

use crate::intent::{Action, Intent};

pub fn parse(text: &str) -> Option<Intent> {
    let tokens: Vec<&str> = text
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| ",;".contains(c)))
        .filter(|token| !token.is_empty())
        .collect();
    let first = tokens.first()?.to_lowercase();
    match first.as_str() {
        "ls" | "dir" => list(&tokens[1..]),
        "pwd" => Some(Intent::new(Action::PrintWorkingDirectory)),
        "whoami" if tokens.len() == 1 => Some(Intent::new(Action::WhoAmI)),
        "hostname" if tokens.len() == 1 => Some(Intent::new(Action::ShowHostname)),
        "date" if tokens.len() == 1 => Some(Intent::new(Action::ShowDate)),
        "uptime" if tokens.len() == 1 => Some(Intent::new(Action::ShowUptime)),
        "sw_vers" | "sysinfo" if tokens.len() == 1 => Some(Intent::new(Action::ShowSystemInfo)),
        "memory" | "meminfo" if tokens.len() == 1 => Some(Intent::new(Action::ShowMemory)),
        "cpu" | "cpuinfo" if tokens.len() == 1 => Some(Intent::new(Action::ShowCpu)),
        "network" | "netinfo" if tokens.len() == 1 => Some(Intent::new(Action::ShowNetwork)),
        "ps" if tokens.len() == 1 => Some(Intent::new(Action::ListProcesses)),
        "df" if tokens.len() <= 2 => Some(Intent::new(Action::ShowDiskUsage)),
        "cat" | "head" | "type" => read(&tokens),
        "mkdir" | "md" => create(&tokens[1..]),
        "cp" => copy(&tokens[1..]),
        "mv" => relocate(Action::MoveFile, &tokens[1..]),
        "rm" | "rmdir" => remove(&tokens),
        "cd" | "chdir" => change_dir(&tokens[1..]),
        "du" => size(&tokens[1..]),
        "which" | "whereis" => which(&tokens[1..]),
        "kill" => kill(&tokens[1..]),
        "lsof" => lsof(&tokens[1..]),
        "open" if tokens.len() >= 2 && tokens[1].starts_with('-') => None,
        _ => None,
    }
}

fn list(tokens: &[&str]) -> Option<Intent> {
    let mut path = None;
    let mut long = false;
    let mut hidden = false;
    for token in tokens {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("./") {
            if token.starts_with("--") {
                return None;
            }
            for letter in token[1..].chars() {
                match letter {
                    'l' => long = true,
                    'a' => hidden = true,
                    'h' | '1' | 'F' => {}
                    _ => return None,
                }
            }
        } else if path.is_some() {
            return None;
        } else {
            path = Some(*token);
        }
    }
    let mut intent = match path {
        Some(path) => Intent::target(Action::ListDirectory, path),
        None => Intent::new(Action::ListDirectory),
    };
    intent.long = long;
    intent.hidden = hidden;
    Some(intent)
}

fn read(tokens: &[&str]) -> Option<Intent> {
    let first = tokens.first()?.to_lowercase();
    let rest = &tokens[1..];
    let mut limit = None;
    let mut paths: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let token = rest[i];
        if first == "head" && (token == "-n" || token == "--lines") {
            let count = rest.get(i + 1)?.parse::<usize>().ok()?;
            limit = Some(count);
            i += 2;
            continue;
        }
        if first == "head"
            && token.starts_with('-')
            && token[1..].chars().all(|c| c.is_ascii_digit())
        {
            limit = token[1..].parse().ok();
            i += 1;
            continue;
        }
        if token.starts_with('-') {
            return None;
        }
        paths.push(token);
        i += 1;
    }
    if paths.len() != 1 {
        return None;
    }
    let mut intent = Intent::target(Action::ReadFile, paths[0]);
    intent.limit = limit;
    Some(intent)
}

fn create(tokens: &[&str]) -> Option<Intent> {
    let mut paths = Vec::new();
    for token in tokens {
        if *token == "-p" || *token == "--parents" {
            continue;
        }
        if token.starts_with('-') {
            return None;
        }
        paths.push(*token);
    }
    (paths.len() == 1).then(|| Intent::target(Action::CreateDirectory, paths[0]))
}

fn copy(tokens: &[&str]) -> Option<Intent> {
    relocate(Action::CopyFile, tokens)
}

fn relocate(action: Action, tokens: &[&str]) -> Option<Intent> {
    let paths: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|token| !token.starts_with('-'))
        .collect();
    if tokens.iter().any(|token| token.starts_with('-')) {
        return None;
    }
    if paths.len() != 2 {
        return None;
    }
    let mut intent = Intent::new(action);
    intent.source = Some(paths[0].to_string());
    intent.destination = Some(paths[1].to_string());
    Some(intent)
}

fn remove(tokens: &[&str]) -> Option<Intent> {
    let command = *tokens.first()?;
    let args = &tokens[1..];
    let recursive = command == "rmdir"
        || args
            .iter()
            .any(|token| matches!(*token, "-r" | "-rf" | "-fr" | "-R"));
    let paths: Vec<&str> = args
        .iter()
        .copied()
        .filter(|token| {
            *token != "-r"
                && *token != "-f"
                && *token != "-rf"
                && *token != "-fr"
                && *token != "-R"
                && *token != "--"
        })
        .collect();
    if paths.iter().any(|token| token.starts_with('-')) || paths.len() != 1 {
        return None;
    }
    let target = paths[0];
    let action = if recursive {
        Action::DeleteDirectory
    } else if target.contains('/') || target.contains('.') || target == "node_modules" {
        Action::DeleteFile
    } else {
        Action::RemoveApp
    };
    Some(Intent::target(action, target))
}

fn change_dir(tokens: &[&str]) -> Option<Intent> {
    let rest = tokens
        .iter()
        .copied()
        .filter(|token| {
            ![
                "to",
                "in",
                "into",
                "at",
                "в",
                "на",
                "до",
                "do",
                "a",
                "the",
                "folder",
                "dir",
                "папку",
                "carpeta",
                "ordner",
                "dossier",
                "pasta",
                "cartella",
                "klasör",
                "map",
                "إلى",
                "الى",
                "में",
            ]
            .contains(&token.to_lowercase().as_str())
        })
        .collect::<Vec<_>>();
    if rest.is_empty() {
        return Some(Intent::target(Action::ChangeDirectory, "~"));
    }
    if rest.len() != 1 || rest[0].starts_with('-') {
        return None;
    }
    Some(Intent::target(Action::ChangeDirectory, rest[0]))
}

fn size(tokens: &[&str]) -> Option<Intent> {
    let paths: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|token| !token.starts_with('-'))
        .collect();
    let target = if paths.is_empty() { "." } else { paths[0] };
    if paths.len() > 1 {
        return None;
    }
    Some(Intent::target(Action::DirectorySize, target))
}

fn which(tokens: &[&str]) -> Option<Intent> {
    (tokens.len() == 1 && !tokens[0].starts_with('-'))
        .then(|| Intent::target(Action::FindProcess, tokens[0]))
}

fn kill(tokens: &[&str]) -> Option<Intent> {
    let pid = tokens
        .iter()
        .find_map(|token| token.parse::<u32>().ok())
        .filter(|pid| *pid > 0)?;
    if tokens.iter().any(|token| {
        [
            "port",
            "порт",
            "порту",
            "listening",
            "process",
            "процесс",
            "-term",
            "-9",
            "-kill",
        ]
        .contains(&token.to_lowercase().as_str())
    }) {
        return None;
    }
    if tokens.len() > 2 {
        return None;
    }
    let mut intent = Intent::new(Action::KillProcess);
    intent.pid = Some(pid);
    Some(intent)
}

fn lsof(tokens: &[&str]) -> Option<Intent> {
    for token in tokens {
        let lower = token.to_lowercase();
        let number = lower
            .rsplit(|c: char| !c.is_ascii_digit())
            .find(|part| !part.is_empty())
            .and_then(|part| part.parse::<u16>().ok())
            .filter(|port| *port > 0);
        if let Some(port) = number
            && (lower.contains("tcp") || lower.contains(":") || lower.contains("i"))
        {
            let mut intent = Intent::new(Action::FindPortProcess);
            intent.port = Some(port);
            return Some(intent);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_commands_are_typed() {
        let ls = parse("ls -la Downloads").unwrap();
        assert_eq!(ls.action, Action::ListDirectory);
        assert_eq!(ls.target.as_deref(), Some("Downloads"));
        assert!(ls.long && ls.hidden);
        assert_eq!(parse("pwd").unwrap().action, Action::PrintWorkingDirectory);
        assert_eq!(parse("cat README.md").unwrap().action, Action::ReadFile);
        assert_eq!(
            parse("mkdir notes").unwrap().action,
            Action::CreateDirectory
        );
        let copy = parse("cp a.txt b.txt").unwrap();
        assert_eq!(copy.action, Action::CopyFile);
        assert_eq!(copy.source.as_deref(), Some("a.txt"));
        assert_eq!(copy.destination.as_deref(), Some("b.txt"));
        assert_eq!(
            parse("cd Downloads").unwrap().action,
            Action::ChangeDirectory
        );
        assert_eq!(parse("whoami").unwrap().action, Action::WhoAmI);
        assert_eq!(parse("uptime").unwrap().action, Action::ShowUptime);
        assert_eq!(parse("sysinfo").unwrap().action, Action::ShowSystemInfo);
        assert_eq!(parse("memory").unwrap().action, Action::ShowMemory);
        assert_eq!(parse("cpu").unwrap().action, Action::ShowCpu);
        assert_eq!(parse("network").unwrap().action, Action::ShowNetwork);
        assert_eq!(parse("ps").unwrap().action, Action::ListProcesses);
        let kill = parse("kill 123").unwrap();
        assert_eq!(kill.action, Action::KillProcess);
        assert_eq!(kill.pid, Some(123));
        assert!(parse("cat catalog").is_some());
        assert!(
            parse("rm -rf /").is_none()
                || parse("rm -rf /").unwrap().target.as_deref() == Some("/")
        );
        assert_eq!(
            parse("rm -r build").unwrap().action,
            Action::DeleteDirectory
        );
        assert_eq!(
            parse("rmdir build").unwrap().action,
            Action::DeleteDirectory
        );
        assert!(parse("ls --evil").is_none());
        assert!(parse("cp -r a b").is_none());
    }
}
