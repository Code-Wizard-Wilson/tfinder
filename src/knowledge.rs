//! Data-driven phrase rewrites for wording that is broader than the compact
//! deterministic lexicon. Rewrites still go through the normal typed parser;
//! this module never executes a command.

use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf, sync::OnceLock};

const BUILTIN: &str = include_str!("../knowledge/phrases.json");
const PLACEHOLDER: &str = "{target}";
const MAX_RULES: usize = 2_048;
const MAX_TERMS: usize = 8_192;
const MAX_PREFIXES: usize = 512;
const MAX_TEXT: usize = 4_096;
const CANONICAL_TERMS: &[&str] = &[
    "open", "delete", "find", "move", "rename", "clear", "kill", "size", "list", "copy", "read",
    "mkdir", "cd",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeFile {
    version: u32,
    #[serde(default)]
    prefixes: Vec<String>,
    #[serde(default)]
    terms: HashMap<String, Vec<String>>,
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    #[serde(rename = "match")]
    pattern: String,
    rewrite: String,
}

#[derive(Debug, Clone)]
struct Knowledge {
    prefixes: Vec<String>,
    terms: HashMap<String, String>,
    max_term_words: usize,
    rules: Vec<Rule>,
    rule_index: HashMap<String, Vec<usize>>,
}

static BUILTIN_KNOWLEDGE: OnceLock<Knowledge> = OnceLock::new();

pub fn path() -> Result<PathBuf, String> {
    Ok(crate::system::home()
        .map_err(|error| error.to_string())?
        .join(".config/terfinder/knowledge.json"))
}

fn parse(content: &str) -> Result<Knowledge, String> {
    let file: KnowledgeFile = serde_json::from_str(content)
        .map_err(|error| format!("invalid knowledge JSON: {error}"))?;
    if file.version != 1 {
        return Err(format!(
            "unsupported knowledge version {}; expected 1",
            file.version
        ));
    }
    if file.rules.len() > MAX_RULES {
        return Err(format!("knowledge file has more than {MAX_RULES} rules"));
    }
    for (index, rule) in file.rules.iter().enumerate() {
        validate_rule(rule).map_err(|error| format!("knowledge rule {}: {error}", index + 1))?;
    }
    let mut term_count = 0;
    let mut terms = HashMap::new();
    let mut max_term_words = 0;
    for (canonical, aliases) in file.terms {
        if !CANONICAL_TERMS.contains(&canonical.as_str()) {
            return Err(format!(
                "unknown canonical term {canonical:?}; expected one of {}",
                CANONICAL_TERMS.join(", ")
            ));
        }
        for alias in aliases {
            let alias = normalized_entry(&alias, "term")?;
            term_count += 1;
            if term_count > MAX_TERMS {
                return Err(format!("knowledge file has more than {MAX_TERMS} terms"));
            }
            max_term_words = max_term_words.max(alias.split_whitespace().count());
            if let Some(previous) = terms.insert(alias.clone(), canonical.clone())
                && previous != canonical
            {
                return Err(format!(
                    "term {alias:?} maps to both {previous:?} and {canonical:?}"
                ));
            }
        }
    }
    if file.prefixes.len() > MAX_PREFIXES {
        return Err(format!(
            "knowledge file has more than {MAX_PREFIXES} prefixes"
        ));
    }
    let mut prefixes = file
        .prefixes
        .iter()
        .map(|prefix| normalized_entry(prefix, "prefix"))
        .collect::<Result<Vec<_>, _>>()?;
    prefixes.sort_by(|left, right| {
        right
            .chars()
            .count()
            .cmp(&left.chars().count())
            .then_with(|| left.cmp(right))
    });
    prefixes.dedup();
    let mut rule_index: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, rule) in file.rules.iter().enumerate() {
        let key = normalize(&rule.pattern)
            .split_whitespace()
            .next()
            .unwrap_or("*")
            .to_lowercase();
        let key = if key.contains(PLACEHOLDER) {
            "*".to_string()
        } else {
            key
        };
        rule_index.entry(key).or_default().push(index);
    }
    Ok(Knowledge {
        prefixes,
        terms,
        max_term_words,
        rules: file.rules,
        rule_index,
    })
}

fn normalized_entry(value: &str, kind: &str) -> Result<String, String> {
    if value.len() > 128 || value.contains(['\n', '\r', '\0']) || value.contains(PLACEHOLDER) {
        return Err(format!(
            "{kind} must be one line without placeholders and at most 128 bytes"
        ));
    }
    let value = normalize(value).to_lowercase();
    if value.is_empty() {
        return Err(format!("{kind} must not be empty"));
    }
    Ok(value)
}

fn validate_rule(rule: &Rule) -> Result<(), String> {
    let pattern = rule.pattern.trim();
    let rewrite = rule.rewrite.trim();
    if pattern.is_empty() || rewrite.is_empty() {
        return Err("match and rewrite must not be empty".into());
    }
    if pattern.len() > MAX_TEXT || rewrite.len() > MAX_TEXT {
        return Err(format!(
            "match and rewrite must be at most {MAX_TEXT} bytes"
        ));
    }
    if pattern.contains(['\n', '\r', '\0']) || rewrite.contains(['\n', '\r', '\0']) {
        return Err("newlines and NUL bytes are not allowed".into());
    }
    let pattern_slots = pattern.matches(PLACEHOLDER).count();
    let rewrite_slots = rewrite.matches(PLACEHOLDER).count();
    if pattern_slots > 1 || rewrite_slots > 1 || pattern_slots != rewrite_slots {
        return Err("use either no placeholder or one {target} in both fields".into());
    }
    Ok(())
}

fn builtin() -> &'static Knowledge {
    BUILTIN_KNOWLEDGE
        .get_or_init(|| parse(BUILTIN).expect("bundled knowledge/phrases.json must be valid"))
}

fn local() -> Result<Knowledge, String> {
    let file = path()?;
    match fs::read_to_string(&file) {
        Ok(content) => parse(&content).map_err(|error| format!("{}: {error}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Knowledge {
            prefixes: Vec::new(),
            terms: HashMap::new(),
            max_term_words: 0,
            rules: Vec::new(),
            rule_index: HashMap::new(),
        }),
        Err(error) => Err(format!("could not read {}: {error}", file.display())),
    }
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .trim_matches(|character: char| ".,!?;:".contains(character))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_chars_at(value: &str, count: usize) -> (&str, &str) {
    if count == 0 {
        return ("", value);
    }
    match value.char_indices().nth(count) {
        Some((index, _)) => value.split_at(index),
        None => (value, ""),
    }
}

fn capture(pattern: &str, input: &str) -> Option<Option<String>> {
    let pattern = normalize(pattern);
    let input = normalize(input);
    let pattern_lower = pattern.to_lowercase();
    let input_lower = input.to_lowercase();
    let Some((prefix, suffix)) = pattern_lower.split_once(PLACEHOLDER) else {
        return (input_lower == pattern_lower).then_some(None);
    };
    if !input_lower.starts_with(prefix) || !input_lower.ends_with(suffix) {
        return None;
    }
    let input_count = input.chars().count();
    let prefix_count = prefix.chars().count();
    let suffix_count = suffix.chars().count();
    if input_count <= prefix_count + suffix_count {
        return None;
    }
    let (_, after_prefix) = split_chars_at(&input, prefix_count);
    let target_count = input_count - prefix_count - suffix_count;
    let (target, _) = split_chars_at(after_prefix, target_count);
    let target = target.trim();
    (!target.is_empty()).then(|| Some(target.to_string()))
}

fn apply(rule: &Rule, input: &str) -> Option<String> {
    let target = capture(&rule.pattern, input)?;
    let output = match target {
        Some(target) => rule.rewrite.replacen(PLACEHOLDER, &target, 1),
        None => rule.rewrite.clone(),
    };
    (output.len() <= MAX_TEXT).then(|| normalize(&output))
}

fn apply_rules(knowledge: &Knowledge, input: &str) -> Option<String> {
    let key = normalize(input).split_whitespace().next()?.to_lowercase();
    knowledge
        .rule_index
        .get(&key)
        .into_iter()
        .chain(knowledge.rule_index.get("*"))
        .flatten()
        .find_map(|index| apply(&knowledge.rules[*index], input))
}

fn strip_prefix<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let lower = input.to_lowercase();
    if lower == prefix {
        return None;
    }
    let tail = lower.strip_prefix(prefix)?;
    if !tail.starts_with(' ') {
        return None;
    }
    let prefix_chars = prefix.chars().count();
    let (_, rest) = split_chars_at(input, prefix_chars);
    Some(rest.trim_start())
}

fn without_politeness<'a>(input: &'a str, local: &Knowledge, builtin: &Knowledge) -> &'a str {
    local
        .prefixes
        .iter()
        .chain(&builtin.prefixes)
        .filter_map(|prefix| strip_prefix(input, prefix).map(|rest| (prefix.chars().count(), rest)))
        .max_by_key(|(length, _)| *length)
        .map(|(_, rest)| rest)
        .unwrap_or(input)
}

fn apply_terms(local: &Knowledge, builtin: &Knowledge, input: &str) -> Option<String> {
    let normalized = normalize(input);
    let input = without_politeness(&normalized, local, builtin);
    let words = input.split_whitespace().collect::<Vec<_>>();
    let max_words = local
        .max_term_words
        .max(builtin.max_term_words)
        .min(words.len());
    for count in (1..=max_words).rev() {
        let alias = words[..count].join(" ").to_lowercase();
        let canonical = local
            .terms
            .get(&alias)
            .or_else(|| builtin.terms.get(&alias));
        if let Some(canonical) = canonical {
            let rest = words[count..].join(" ");
            return Some(if rest.is_empty() {
                canonical.clone()
            } else {
                format!("{canonical} {rest}")
            });
        }
    }
    None
}

/// Expand one complete, anchored phrase. Local rules take precedence over the
/// bundled phrasebook. Only one rewrite is applied so rules cannot loop.
pub fn rewrite(input: &str) -> String {
    if input.len() > MAX_TEXT {
        return input.to_string();
    }
    let normalized = normalize(input);
    let builtin = builtin();
    let Ok(local) = local() else {
        return apply_rules(builtin, &normalized)
            .or_else(|| apply_terms(builtin, builtin, &normalized))
            .unwrap_or_else(|| without_politeness(&normalized, builtin, builtin).to_string());
    };
    apply_rules(&local, &normalized)
        .or_else(|| apply_rules(builtin, &normalized))
        .or_else(|| apply_terms(&local, builtin, &normalized))
        .unwrap_or_else(|| without_politeness(&normalized, &local, builtin).to_string())
}

pub fn print_status() -> Result<(), String> {
    let local = local()?;
    println!("Bundled terms:        {}", builtin().terms.len());
    println!("Bundled phrase rules: {}", builtin().rules.len());
    println!("Local terms:          {}", local.terms.len());
    println!("Local phrase rules:   {}", local.rules.len());
    println!("Local knowledge file: {}", path()?.display());
    println!("Status: valid");
    Ok(())
}

pub fn health_summary() -> Result<String, String> {
    let local = local()?;
    Ok(format!(
        "{} bundled terms, {} bundled rules, {} local terms, {} local rules",
        builtin().terms.len(),
        builtin().rules.len(),
        local.terms.len(),
        local.rules.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_a_target_at_any_position() {
        let rule = Rule {
            pattern: "purge {target} cache".into(),
            rewrite: "clear cache {target}".into(),
        };
        assert_eq!(
            apply(&rule, "Purge npm cache!"),
            Some("clear cache npm".into())
        );
    }

    #[test]
    fn rules_are_anchored() {
        let rule = Rule {
            pattern: "вруби {target}".into(),
            rewrite: "open {target}".into(),
        };
        assert_eq!(apply(&rule, "вруби Telegram"), Some("open Telegram".into()));
        assert_eq!(apply(&rule, "если можно вруби Telegram"), None);
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_schema() {
        assert!(parse(r#"{"version":2,"rules":[]}"#).is_err());
        assert!(
            parse(r#"{"version":1,"rules":[{"match":"x {target}","rewrite":"open y"}]}"#).is_err()
        );
        assert!(parse(r#"{"version":1,"rules":[{"match":"x\ny","rewrite":"pwd"}]}"#).is_err());
        assert!(parse(r#"{"version":1,"terms":{"shell":["do"]}}"#).is_err());
    }

    #[test]
    fn terms_use_longest_match_and_drop_politeness() {
        let knowledge =
            parse(r#"{"version":1,"prefixes":["please"],"terms":{"open":["fire","fire up"]}}"#)
                .unwrap();
        let empty = parse(r#"{"version":1}"#).unwrap();
        assert_eq!(
            apply_terms(&knowledge, &empty, "please fire up Safari"),
            Some("open Safari".into())
        );
    }

    #[test]
    fn every_bundled_term_preserves_its_target() {
        let bundled = builtin();
        let empty = parse(r#"{"version":1}"#).unwrap();
        for (alias, canonical) in &bundled.terms {
            let input = format!("{alias} Sentinel");
            assert_eq!(
                apply_terms(bundled, &empty, &input),
                Some(format!("{canonical} Sentinel")),
                "term {alias:?}"
            );
        }
    }

    #[test]
    fn every_bundled_rule_is_anchored_and_rewrites_exactly() {
        for rule in &builtin().rules {
            let input = rule.pattern.replace(PLACEHOLDER, "Sentinel");
            let expected = normalize(&rule.rewrite.replace(PLACEHOLDER, "Sentinel"));
            assert_eq!(
                apply(rule, &input),
                Some(expected),
                "rule {:?}",
                rule.pattern
            );
            assert_eq!(
                apply(rule, &format!("prefix {input}")),
                None,
                "rule {:?} accepted an unanchored prefix",
                rule.pattern
            );
        }
    }
}
