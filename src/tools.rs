//! Allowlisted git / brew / cargo. Fixed program, checked argv, no shell.

use crate::intent::{Action, Intent};

pub fn parse(text: &str) -> Option<Intent> {
    if text.contains(['$', '`', ';', '|', '\n', '\0', '>', '<', '&']) {
        return None;
    }
    let tokens: Vec<&str> = text
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| ",;".contains(c)))
        .filter(|token| !token.is_empty())
        .collect();
    let first = tokens.first()?.to_lowercase();
    match first.as_str() {
        "git" => git(&tokens[1..]),
        "brew" => brew(&tokens[1..]),
        "cargo" => cargo(&tokens[1..]),
        _ => None,
    }
}

fn dangerous(arg: &str) -> bool {
    arg.contains(['$', '`', ';', '|', '\n', '\0', '>', '<'])
        || arg.starts_with("--exec")
        || arg.starts_with("--config")
        || arg == "-c"
        || arg == "-C"
        || arg == "-x"
        || arg == "--upload-pack"
}

fn git(args: &[&str]) -> Option<Intent> {
    let sub = args.first()?.to_lowercase();
    const OK: &[&str] = &[
        "status",
        "log",
        "diff",
        "branch",
        "remote",
        "show",
        "stash",
        "version",
        "rev-parse",
        "describe",
    ];
    if !OK.contains(&sub.as_str()) {
        return None;
    }
    if args.iter().copied().any(dangerous) {
        return None;
    }
    const FLAGS: &[&str] = &[
        "-u",
        "-v",
        "-vv",
        "-a",
        "-b",
        "-d",
        "-n",
        "-p",
        "-1",
        "--oneline",
        "--stat",
        "--short",
        "--branch",
        "--list",
        "--all",
        "--remotes",
        "--name-only",
        "--cached",
        "--stash",
        "--show",
        "--abbrev-ref",
        "HEAD",
        "--",
    ];
    for (index, arg) in args.iter().enumerate().skip(1) {
        if FLAGS.contains(arg) || arg.parse::<i32>().is_ok() {
            continue;
        }
        if *arg == "-n" || args.get(index.saturating_sub(1)) == Some(&"-n") {
            continue;
        }
        if arg.starts_with('-') {
            return None;
        }
        if arg.contains("..") && arg.contains('/') {
            return None;
        }
    }
    tool("git", args)
}

fn brew(args: &[&str]) -> Option<Intent> {
    let sub = args.first()?.to_lowercase();
    const OK: &[&str] = &[
        "list",
        "outdated",
        "info",
        "search",
        "config",
        "--version",
        "deps",
        "uses",
        "home",
    ];
    if !OK.contains(&sub.as_str()) {
        return None;
    }
    if args.iter().copied().any(dangerous) {
        return None;
    }
    if args.iter().skip(1).any(|arg| {
        arg.starts_with('-') && !matches!(*arg, "--json" | "-v" | "--formula" | "--cask")
    }) {
        return None;
    }
    tool("brew", args)
}

fn cargo(args: &[&str]) -> Option<Intent> {
    let sub = args.first()?.to_lowercase();
    const OK: &[&str] = &[
        "test",
        "check",
        "build",
        "clippy",
        "fmt",
        "tree",
        "metadata",
        "--version",
        "version",
        "fetch",
    ];
    if !OK.contains(&sub.as_str()) {
        return None;
    }
    if args.iter().copied().any(dangerous) {
        return None;
    }
    const FLAGS: &[&str] = &[
        "-q",
        "-v",
        "--offline",
        "--release",
        "--all",
        "--all-targets",
        "--all-features",
        "--lib",
        "--bins",
        "--tests",
        "--check",
        "--message-format=json",
        "--workspace",
    ];
    for arg in args.iter().skip(1) {
        if FLAGS.contains(arg) || *arg == "--" {
            continue;
        }
        if arg.starts_with("--features=") || arg.starts_with("--package=") {
            continue;
        }
        if arg.starts_with('-') {
            return None;
        }
    }
    tool("cargo", args)
}

fn tool(program: &str, args: &[&str]) -> Option<Intent> {
    let mut intent = Intent::target(Action::RunTool, program);
    intent.argv = Some(args.iter().map(|s| (*s).to_string()).collect());
    Some(intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists_common_dev_commands() {
        let git = parse("git status").unwrap();
        assert_eq!(git.action, Action::RunTool);
        assert_eq!(git.target.as_deref(), Some("git"));
        assert_eq!(git.argv.as_deref(), Some(&["status".to_string()][..]));
        assert!(parse("git status --oneline").is_some());
        assert!(parse("git rebase").is_none());
        assert!(parse("git log --exec rm").is_none());
        assert!(parse("brew outdated").is_some());
        assert!(parse("brew install wget").is_none());
        assert!(parse("cargo test --offline").is_some());
        assert!(parse("cargo install ripgrep").is_none());
        for input in [
            "git status; touch /tmp/pwned",
            "git log | sh",
            "cargo test && open /tmp/pwned",
            "brew info $(whoami)",
        ] {
            assert!(parse(input).is_none(), "{input}");
        }
    }
}
