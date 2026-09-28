//! User phrase rewrites and a local miss log. No network.

use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

static ALIASES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn home() -> Option<PathBuf> {
    crate::system::home().ok()
}

pub fn aliases_path() -> Option<PathBuf> {
    Some(home()?.join(".config/terfinder/aliases.toml"))
}

fn misses_path() -> Option<PathBuf> {
    Some(home()?.join(".cache/terfinder/misses.log"))
}

fn parse_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').trim().to_lowercase();
        let value = value.trim().trim_matches('"').trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        map.insert(key, value.to_string());
    }
    map
}

fn load() -> HashMap<String, String> {
    let Ok(mut guard) = ALIASES.lock() else {
        return HashMap::new();
    };
    if let Some(map) = guard.as_ref() {
        return map.clone();
    }
    let map = aliases_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| parse_map(&content))
        .unwrap_or_default();
    *guard = Some(map.clone());
    map
}

fn invalidate() {
    if let Ok(mut guard) = ALIASES.lock() {
        *guard = None;
    }
}

/// Rewrite an input if the user saved an exact-phrase alias.
pub fn rewrite(input: &str) -> String {
    let key = input.trim().to_lowercase();
    if key.is_empty() {
        return input.to_string();
    }
    load()
        .get(&key)
        .cloned()
        .unwrap_or_else(|| input.to_string())
}

pub fn save(phrase: &str, command: &str) -> Result<(), String> {
    let phrase = phrase.trim();
    let command = command.trim();
    if phrase.is_empty() || command.is_empty() {
        return Err("usage: tf remember <phrase> = <command>".into());
    }
    if phrase.len() > 512 || command.len() > crate::fast::MAX_INPUT_BYTES {
        return Err("alias phrase must be at most 512 bytes and command at most 4096 bytes".into());
    }
    if phrase.contains('\n') || command.contains('\n') {
        return Err("alias cannot contain newlines".into());
    }
    let path = aliases_path().ok_or("HOME is not set")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut body = fs::read_to_string(&path).unwrap_or_default();
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str(&format!(
        "\"{}\" = \"{}\"\n",
        phrase.replace('"', "'"),
        command.replace('"', "'")
    ));
    fs::write(&path, body).map_err(|e| e.to_string())?;
    invalidate();
    Ok(())
}

pub fn remember_line(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let (left, right) = raw
        .split_once('=')
        .ok_or("usage: tf remember <phrase> = <command>")?;
    save(left, right)?;
    Ok(format!("Saved alias: {} → {}", left.trim(), right.trim()))
}

pub fn record_miss(input: &str) {
    let Some(path) = misses_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "{secs}\t{}", input.replace('\n', " "));
}

pub fn print_misses() -> Result<(), String> {
    let path = misses_path().ok_or("HOME is not set")?;
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("No missed phrases yet.");
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut order = Vec::new();
    for line in content.lines().filter(|line| !line.is_empty()) {
        let text = line.split_once('\t').map(|(_, text)| text).unwrap_or(line);
        let key = text.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        if !counts.contains_key(&key) {
            order.push(key.clone());
        }
        *counts.entry(key).or_default() += 1;
    }
    order.retain(|phrase| crate::fast::parse_outcome(phrase).is_none());
    if order.is_empty() {
        println!("No unresolved missed phrases.");
        return Ok(());
    }
    println!("Unresolved missed phrases:");
    order.sort_by(|left, right| {
        counts[right]
            .cmp(&counts[left])
            .then_with(|| left.cmp(right))
    });
    for text in order.into_iter().take(20) {
        println!("  {:>3}×  {text}", counts[&text]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_alias_lines() {
        let map = parse_map(
            "# comment\n\"почисти докер\" = \"очисти кеш docker\"\nstand = open http://localhost:3000\n",
        );
        assert_eq!(
            map.get("почисти докер").map(String::as_str),
            Some("очисти кеш docker")
        );
        assert_eq!(
            map.get("stand").map(String::as_str),
            Some("open http://localhost:3000")
        );
    }

    #[test]
    fn miss_keys_are_case_insensitive() {
        let content = "1\tHello\n2\thello\n";
        let keys = content
            .lines()
            .filter_map(|line| line.split_once('\t').map(|(_, text)| text.to_lowercase()))
            .collect::<Vec<_>>();
        assert_eq!(keys, ["hello", "hello"]);
    }
}
