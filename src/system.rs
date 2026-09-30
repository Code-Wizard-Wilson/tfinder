use crate::intent::FileType;
use std::{
    collections::HashSet,
    env,
    ffi::CString,
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
    time::{Duration, Instant, SystemTime},
};

pub fn home() -> io::Result<PathBuf> {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    if let Some(home) = HOME.get() {
        return Ok(home.clone());
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("HOME is not set"))?;
    let _ = HOME.set(home.clone());
    Ok(home)
}

pub fn expand(value: &str) -> io::Result<PathBuf> {
    let home = home()?;
    let value = value.trim();
    if value.is_empty() {
        return Err(io::Error::other("empty path"));
    }
    if value == "hosts" {
        return Ok(PathBuf::from("/etc/hosts"));
    }
    if value == "~" {
        return Ok(home);
    }
    if let Some(tail) = value.strip_prefix("~/") {
        return Ok(home.join(tail));
    }
    if let Some(directory) = ["Desktop", "Documents", "Downloads"]
        .into_iter()
        .find(|directory| value.eq_ignore_ascii_case(directory))
    {
        return Ok(home.join(directory));
    }
    Ok(PathBuf::from(value))
}

pub fn normalize(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    let mut out = PathBuf::new();
    for part in absolute.components() {
        use std::path::Component;
        match part {
            Component::RootDir => out.push("/"),
            Component::Normal(s) => out.push(s),
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) => return Err(io::Error::other("unsupported path prefix")),
        }
    }
    Ok(out)
}

/// An existing directory for a user-supplied name: the literal path (as given,
/// relative to the current directory or under home), or home itself when the
/// name matches the home folder (`Anton` on a Mac named anton).
pub fn existing_dir(target: &str) -> Option<PathBuf> {
    let value = target
        .trim()
        .trim_start_matches(|c: char| "\"'".contains(c))
        .trim_end_matches(|c: char| "?!,.;:\"'".contains(c));
    if value.is_empty() {
        return None;
    }
    let home = home().ok()?;
    let mut candidates = Vec::new();
    if let Ok(expanded) = expand(value) {
        candidates.push(expanded);
    }
    candidates.push(home.join(value));
    for candidate in candidates {
        if let Ok(path) = normalize(&candidate)
            && path.is_dir()
        {
            return Some(path);
        }
    }
    if !value.contains(['/', '~'])
        && home
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(value))
    {
        return Some(home);
    }
    None
}

pub fn protected(path: &Path) -> bool {
    let Ok(home) = home() else { return true };
    let Ok(path) = normalize(path) else {
        return true;
    };
    [
        Path::new("/"),
        Path::new("/System"),
        Path::new("/Library"),
        Path::new("/Applications"),
        Path::new("/Users"),
        home.as_path(),
    ]
    .iter()
    .any(|root| *root == path)
}

pub fn checked_existing(value: &str) -> io::Result<PathBuf> {
    let path = canonical_parent(&normalize(&expand(value)?)?)?;
    if protected(&path) {
        return Err(io::Error::other("protected broad path"));
    }
    fs::symlink_metadata(&path)?;
    Ok(path)
}

pub fn canonical_parent(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("path has no name"))?;
    Ok(fs::canonicalize(parent)?.join(name))
}

pub fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    unsafe extern "C" {
        fn renamex_np(
            source: *const std::ffi::c_char,
            destination: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    const RENAME_EXCL: u32 = 0x00000004;
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::other("NUL in source path"))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| io::Error::other("NUL in destination path"))?;
    if unsafe { renamex_np(source.as_ptr(), destination.as_ptr(), RENAME_EXCL) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TrashResult {
    Path(PathBuf),
    Finder,
}

fn finder_can_trash(path: &Path) -> bool {
    path.starts_with("/Applications")
        && path.extension().is_some_and(|extension| extension == "app")
}

fn trash_with_finder(path: &Path) -> io::Result<()> {
    // argv keeps the path out of the AppleScript source, so quotes and other
    // characters in an app name cannot alter the script.
    let output = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "tell application \"Finder\" to delete POSIX file (item 1 of argv)",
            "-e",
            "end run",
            "--",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if detail.is_empty() {
            format!("osascript exited with {}", output.status)
        } else {
            detail
        };
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Finder could not move the app to Trash: {detail}. Allow your terminal under System Settings → Privacy & Security → App Management, then try again"
            ),
        ));
    }
    if fs::symlink_metadata(path).is_ok() {
        return Err(io::Error::other(
            "Finder returned success, but the application is still in place",
        ));
    }
    Ok(())
}

pub fn trash(path: &Path) -> io::Result<TrashResult> {
    if protected(path) {
        return Err(io::Error::other("protected broad path"));
    }
    fs::symlink_metadata(path)?;
    let trash = home()?.join(".Trash");
    fs::create_dir_all(&trash)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("invalid target name"))?;
    for index in 0..10_000 {
        let candidate = if index == 0 {
            trash.join(name)
        } else {
            let suffix = format!(".terfinder-{index}");
            trash.join(format!("{}{}", name.to_string_lossy(), suffix))
        };
        match rename_no_replace(path, &candidate) {
            Ok(()) => return Ok(TrashResult::Path(candidate)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied && finder_can_trash(path) =>
            {
                trash_with_finder(path)?;
                return Ok(TrashResult::Finder);
            }
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("could not choose a free Trash name"))
}

pub fn command(program: &str, args: &[&str]) -> io::Result<Output> {
    Command::new(program).args(args).output()
}

fn successful_output(program: &str, args: &[&str]) -> io::Result<String> {
    let output = command(program, args)?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if error.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            error
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn bluetooth_power() -> io::Result<bool> {
    Ok(successful_output("blueutil", &["-p"])? == "1")
}

fn parse_wifi_device(output: &str) -> Option<String> {
    let mut wifi = false;
    for line in output.lines() {
        let line = line.trim();
        if let Some(port) = line.strip_prefix("Hardware Port:") {
            wifi = port.trim() == "Wi-Fi";
        } else if wifi && let Some(device) = line.strip_prefix("Device:") {
            return Some(device.trim().to_string());
        }
    }
    None
}

pub fn wifi_device() -> io::Result<String> {
    parse_wifi_device(&successful_output(
        "/usr/sbin/networksetup",
        &["-listallhardwareports"],
    )?)
    .ok_or_else(|| io::Error::other("Wi-Fi hardware port was not found"))
}

pub fn wifi_power(device: &str) -> io::Result<bool> {
    let output = successful_output("/usr/sbin/networksetup", &["-getairportpower", device])?;
    Ok(output.ends_with(": On"))
}

pub fn set_wifi_power(device: &str, on: bool) -> io::Result<()> {
    successful_output(
        "/usr/sbin/networksetup",
        &["-setairportpower", device, if on { "on" } else { "off" }],
    )?;
    Ok(())
}

pub fn airdrop_mode() -> io::Result<String> {
    match successful_output(
        "/usr/bin/defaults",
        &["read", "com.apple.sharingd", "DiscoverableMode"],
    ) {
        Ok(mode) if matches!(mode.as_str(), "Off" | "Contacts Only" | "Everyone") => Ok(mode),
        Ok(mode) => Err(io::Error::other(format!(
            "unknown AirDrop discoverable mode: {mode}"
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(error),
        Err(_) => Ok("Off".into()),
    }
}

pub fn set_airdrop_mode(mode: &str) -> io::Result<()> {
    if !matches!(mode, "Off" | "Contacts Only" | "Everyone") {
        return Err(io::Error::other("invalid AirDrop mode"));
    }
    successful_output(
        "/usr/bin/defaults",
        &[
            "write",
            "com.apple.sharingd",
            "DiscoverableMode",
            "-string",
            mode,
        ],
    )?;
    // sharingd is a per-user launch service and restarts automatically. This
    // makes the new receiving mode take effect immediately.
    let _ = command("/usr/bin/killall", &["sharingd"]);
    Ok(())
}

pub fn stage_manager_enabled() -> io::Result<bool> {
    match successful_output(
        "/usr/bin/defaults",
        &["read", "com.apple.WindowManager", "GloballyEnabled"],
    ) {
        Ok(value) => Ok(value == "1"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(error),
        Err(_) => Ok(false),
    }
}

pub fn set_stage_manager(on: bool) -> io::Result<()> {
    successful_output(
        "/usr/bin/defaults",
        &[
            "write",
            "com.apple.WindowManager",
            "GloballyEnabled",
            "-bool",
            if on { "true" } else { "false" },
        ],
    )?;
    Ok(())
}

pub fn applications() -> io::Result<Vec<PathBuf>> {
    let roots = [
        PathBuf::from("/Applications"),
        home()?.join("Applications"),
        PathBuf::from("/System/Applications"),
    ];
    let stamp = app_roots_stamp(&roots);
    if let Some(cached) = read_app_cache(stamp) {
        return Ok(cached);
    }
    let mut apps = Vec::new();
    let mut pending: Vec<(PathBuf, u8)> = roots.into_iter().map(|root| (root, 0)).collect();
    while let Some((directory, depth)) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "app") {
                apps.push(path);
                continue;
            }
            if depth < 2 && path.is_dir() {
                pending.push((path, depth + 1));
            }
        }
    }
    apps.sort();
    let _ = write_app_cache(stamp, &apps);
    Ok(apps)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCleanupItem {
    pub category: &'static str,
    pub path: PathBuf,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCleanup {
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub items: Vec<AppCleanupItem>,
}

fn plist_string(path: &Path, key: &str) -> Option<String> {
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", &format!("Print :{key}")])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn app_identity(app: &Path) -> (String, Option<String>, Vec<String>) {
    let info = app.join("Contents/Info.plist");
    let stem = app
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let name = plist_string(&info, "CFBundleName").unwrap_or_else(|| stem.clone());
    let bundle_id = plist_string(&info, "CFBundleIdentifier");
    let mut terms = stem
        .split(|character: char| !character.is_alphanumeric())
        .chain(name.split(|character: char| !character.is_alphanumeric()))
        .map(str::to_lowercase)
        .filter(|term| term.chars().count() >= 4)
        .filter(|term| !matches!(term.as_str(), "application" | "desktop" | "client"))
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    (name, bundle_id, terms)
}

fn matches_app_leftover(
    file_name: &str,
    app_name: &str,
    bundle_id: Option<&str>,
    terms: &[String],
) -> bool {
    let candidate = file_name.to_lowercase();
    let app_name = app_name.to_lowercase();
    let exact_name = candidate == app_name
        || candidate
            .strip_suffix(".plist")
            .is_some_and(|value| value == app_name)
        || candidate
            .strip_suffix(".savedstate")
            .is_some_and(|value| value == app_name);
    let identifier = bundle_id.is_some_and(|identifier| {
        let identifier = identifier.to_lowercase();
        candidate == identifier
            || candidate
                .strip_prefix(&identifier)
                .is_some_and(|suffix| suffix.starts_with('.'))
    });
    exact_name
        || identifier
        || (!terms.is_empty() && terms.iter().all(|term| candidate.contains(term)))
}

fn launch_agent_matches(path: &Path, app: &Path, bundle_id: Option<&str>) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() > 1_048_576 {
        return false;
    }
    let Ok(content) = fs::read(path) else {
        return false;
    };
    let content = String::from_utf8_lossy(&content).to_lowercase();
    let app_path = app.to_string_lossy().to_lowercase();
    content.contains(&app_path)
        || bundle_id.is_some_and(|identifier| content.contains(&identifier.to_lowercase()))
}

/// Build a conservative, user-scoped uninstall plan. Only direct children of
/// known ~/Library locations are considered; shared Group Containers and
/// system-wide /Library helpers are deliberately excluded.
pub fn app_cleanup(app: &Path) -> io::Result<AppCleanup> {
    if app.extension().is_none_or(|extension| extension != "app") {
        return Err(io::Error::other("target is not an application bundle"));
    }
    fs::symlink_metadata(app)?;
    let (app_name, bundle_id, terms) = app_identity(app);
    let home = home()?;
    let library = home.join("Library");
    let roots = [
        ("Application Support", library.join("Application Support")),
        ("Cache", library.join("Caches")),
        ("Preferences", library.join("Preferences")),
        ("Saved State", library.join("Saved Application State")),
        ("HTTP Storage", library.join("HTTPStorages")),
        ("WebKit Data", library.join("WebKit")),
        ("Logs", library.join("Logs")),
        ("Application Scripts", library.join("Application Scripts")),
        ("Container", library.join("Containers")),
        ("Launch Agent", library.join("LaunchAgents")),
    ];
    let mut items = vec![AppCleanupItem {
        category: "Application",
        path: app.to_path_buf(),
        bytes: size(app, 500_000).ok(),
    }];
    let mut seen = HashSet::new();
    seen.insert(app.to_path_buf());
    for (category, root) in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.take(5_000).flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let matches_name = matches_app_leftover(&name, &app_name, bundle_id.as_deref(), &terms);
            let matches_agent = category == "Launch Agent"
                && launch_agent_matches(&path, app, bundle_id.as_deref());
            if (matches_name || matches_agent) && seen.insert(path.clone()) {
                items.push(AppCleanupItem {
                    category,
                    bytes: size(&path, 250_000).ok(),
                    path,
                });
            }
        }
    }
    items[1..].sort_by(|left, right| {
        left.category
            .cmp(right.category)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(AppCleanup {
        app_name,
        bundle_id,
        items,
    })
}

fn app_cache_path() -> io::Result<PathBuf> {
    Ok(home()?.join(".cache/terfinder/apps.idx"))
}

fn app_roots_stamp(roots: &[PathBuf]) -> u64 {
    roots.iter().fold(0u64, |acc, root| {
        let secs = fs::metadata(root)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        acc.wrapping_mul(1_000_003).wrapping_add(secs)
    })
}

fn read_app_cache(stamp: u64) -> Option<Vec<PathBuf>> {
    let text = fs::read_to_string(app_cache_path().ok()?).ok()?;
    let mut lines = text.lines();
    if lines.next()? != "v1" {
        return None;
    }
    if lines.next()?.parse::<u64>().ok()? != stamp {
        return None;
    }
    Some(
        lines
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect(),
    )
}

fn write_app_cache(stamp: u64, apps: &[PathBuf]) -> io::Result<()> {
    let path = app_cache_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = format!("v1\n{stamp}\n");
    for app in apps {
        body.push_str(&app.to_string_lossy());
        body.push('\n');
    }
    fs::write(path, body)
}

pub fn list_dir(path: &Path, hidden: bool, long: bool) -> io::Result<String> {
    let mut entries: Vec<_> = fs::read_dir(path)?.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    let mut lines = Vec::new();
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !hidden && name.starts_with('.') {
            continue;
        }
        if long {
            let meta = entry.metadata().ok();
            let (kind, size) = match &meta {
                Some(meta) if meta.is_dir() => ("dir ", 0),
                Some(meta) => ("file", meta.len()),
                None => ("?", 0),
            };
            lines.push(format!("{kind} {size:>12}  {name}"));
        } else {
            lines.push(name.into_owned());
        }
    }
    Ok(lines.join("\n"))
}

pub fn read_text_file(path: &Path, max_bytes: usize) -> io::Result<String> {
    let bytes = fs::read(path)?;
    if bytes.contains(&0) {
        return Err(io::Error::other("refusing to print a binary file"));
    }
    if bytes.len() > max_bytes {
        let mut text = String::from_utf8_lossy(&bytes[..max_bytes]).into_owned();
        text.push_str("\n… truncated …");
        return Ok(text);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn copy_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() || fs::symlink_metadata(destination).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination exists",
        ));
    }
    fs::copy(source, destination)?;
    Ok(())
}

pub fn create_directory(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination exists",
        ));
    }
    fs::create_dir(path)
}

fn app_name(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

fn edit_distance(a: &[char], b: &[char], max: usize) -> usize {
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, left) in a.iter().enumerate() {
        current[0] = i + 1;
        let mut row_best = current[0];
        for (j, right) in b.iter().enumerate() {
            let cost = usize::from(left != right);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
            row_best = row_best.min(current[j + 1]);
        }
        if row_best > max {
            return max + 1;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

fn app_score(name: &str, needle: &str) -> u8 {
    if name == needle {
        return 100;
    }
    if let Some(stripped) = name.strip_prefix("google ")
        && stripped == needle
    {
        return 95;
    }
    if name.starts_with(needle) {
        return 85;
    }
    let name_words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect();
    if name_words.contains(&needle) {
        return 80;
    }
    if name.contains(needle) {
        return 75;
    }
    let query_words: Vec<&str> = needle.split_whitespace().collect();
    if query_words.len() > 1 && query_words.iter().all(|word| name_words.contains(word)) {
        return 72;
    }
    let name_chars: Vec<char> = name.chars().collect();
    let query_chars: Vec<char> = needle.chars().collect();
    let shortest = name_chars.len().min(query_chars.len());
    let budget = if shortest >= 8 {
        2
    } else if shortest >= 5 {
        1
    } else {
        0
    };
    if budget > 0 && edit_distance(&name_chars, &query_chars, budget) <= budget {
        return 55;
    }
    if shortest >= 5
        && name_words
            .iter()
            .any(|word| edit_distance(&word.chars().collect::<Vec<_>>(), &query_chars, 1) <= 1)
    {
        return 50;
    }
    0
}

/// Resolve an app query to installed bundles. Spoken names, typos and partial
/// names all resolve; ties are kept so the caller can ask the user to choose.
pub fn find_apps(query: &str) -> io::Result<Vec<PathBuf>> {
    let lowered = query.trim().to_lowercase();
    let base = lowered.strip_suffix(".app").unwrap_or(&lowered);
    let needle = crate::lexicon::canonical_app_query(base)
        .trim_matches(|c: char| "!?,.:;".contains(c))
        .to_string();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let mut scored: Vec<(u8, PathBuf)> = Vec::new();
    for path in applications()? {
        let score = app_score(&app_name(&path), &needle);
        if score > 0 {
            scored.push((score, path));
        }
    }
    if scored.is_empty() {
        return Ok(Vec::new());
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let best = scored[0].0;
    Ok(scored
        .into_iter()
        .take_while(|(score, _)| *score == best)
        .map(|(_, path)| path)
        .collect())
}

/// The longest installed app whose name appears verbatim in the request.
pub fn mentioned_app(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for path in applications().ok()? {
        let name = path.file_stem()?.to_string_lossy().to_string();
        let needle = name.to_lowercase();
        if !needle.is_empty()
            && lower.contains(&needle)
            && best
                .as_ref()
                .is_none_or(|(length, _)| needle.len() > *length)
        {
            best = Some((needle.len(), name));
        }
    }
    best.map(|(_, name)| name)
}

/// Running processes whose executable lives inside an app bundle.
pub fn app_processes(app: &Path) -> Vec<(u32, String)> {
    let prefix = format!("{}/", app.display());
    let target = app.display().to_string();
    let Ok(output) = command("ps", &["-axo", "pid=,comm="]) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        let Some((pid, comm)) = line.split_once(' ') else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        if !safe_pid(pid) {
            continue;
        }
        if comm.starts_with(&prefix) || comm == target {
            found.push((pid, comm.to_string()));
        }
    }
    found.sort();
    found
}

/// Running processes whose executable name matches a query. Used to stop a
/// process by name when no app bundle matches.
pub fn running_processes(query: &str) -> Vec<(u32, String)> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let Ok(output) = command("ps", &["-axo", "pid=,comm="]) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        let Some((pid, comm)) = line.split_once(' ') else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        if !safe_pid(pid) {
            continue;
        }
        let executable = Path::new(comm)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new(comm))
            .to_string_lossy()
            .to_lowercase();
        if executable == needle || (needle.len() >= 4 && executable.contains(&needle)) {
            found.push((pid, comm.to_string()));
        }
    }
    found.sort();
    found
}

pub fn find_files(query: &str) -> io::Result<Vec<PathBuf>> {
    let path = expand(query)?;
    if path.exists() {
        return Ok(vec![normalize(&path)?]);
    }
    let home = home()?;
    let mut hits = Vec::new();
    for directory in [
        env::current_dir()?,
        home.join("Desktop"),
        home.join("Documents"),
        home.join("Downloads"),
    ] {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(query)
                {
                    hits.push(entry.path());
                }
            }
        }
    }
    Ok(hits)
}

fn config_name_variants(name: &str) -> Vec<String> {
    let name = name.trim().to_lowercase();
    let mut variants = vec![
        name.clone(),
        name.replace(' ', "-"),
        name.replace(' ', "_"),
        name.replace(' ', ""),
        name.replace('-', ""),
        name.replace('_', ""),
    ];
    variants.sort();
    variants.dedup();
    variants.retain(|value| !value.is_empty());
    variants
}

fn skip_config_entry(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "bin"
            | ".git"
            | "bun.lock"
            | "package-lock.json"
            | "package.json"
            | ".gitignore"
            | ".DS_Store"
    ) || name.ends_with(".lock")
}

fn looks_like_config_file(name: &str) -> bool {
    let name = name.to_lowercase();
    name.ends_with(".json")
        || name.ends_with(".jsonc")
        || name.ends_with(".toml")
        || name.ends_with(".yaml")
        || name.ends_with(".yml")
        || name.ends_with(".conf")
        || name.ends_with(".cfg")
        || name.ends_with(".ini")
        || name.ends_with(".plist")
        || name.ends_with(".log")
        || name.ends_with("rc")
        || name.ends_with("config")
        || name.starts_with("config")
        || name.starts_with("settings")
        || name == "config"
}

fn push_unique(hits: &mut Vec<PathBuf>, path: PathBuf) {
    if !hits.iter().any(|existing| existing == &path) {
        hits.push(path);
    }
}

fn collect_config_dir(dir: &Path, hits: &mut Vec<PathBuf>) {
    push_unique(hits, dir.to_path_buf());
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skip_config_entry(&name) || name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_file() && looks_like_config_file(&name) {
            push_unique(hits, path);
        }
    }
}

/// Locate config files and folders for a tool name under home, `~/.config`,
/// and `~/Library/Application Support`. The name is supplied by the user;
/// there is no per-app table.
pub fn find_config(name: &str) -> io::Result<Vec<PathBuf>> {
    find_config_in(&home()?, name)
}

pub fn find_config_in(home: &Path, name: &str) -> io::Result<Vec<PathBuf>> {
    let name = name.trim().trim_matches(|c: char| "!?,.;:\"'".contains(c));
    if name.is_empty() || name.contains('/') || name == "~" {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    let support = home.join("Library/Application Support");
    for variant in config_name_variants(name) {
        let dotted = format!(".{variant}");
        let candidates = [
            home.join(".config").join(&variant),
            home.join(&dotted),
            home.join(format!(".{variant}rc")),
            home.join(format!(".{variant}config")),
            home.join(".config").join(format!("{variant}.json")),
            home.join(".config").join(format!("{variant}.jsonc")),
            home.join(".config").join(format!("{variant}.toml")),
            home.join(".config").join(format!("{variant}.yaml")),
            home.join(".config").join(format!("{variant}.yml")),
            home.join(format!(".{variant}.json")),
            home.join(format!(".{variant}.toml")),
            support.join(&variant),
        ];
        for candidate in candidates {
            let Ok(meta) = fs::symlink_metadata(&candidate) else {
                continue;
            };
            if meta.is_dir() {
                collect_config_dir(&candidate, &mut hits);
            } else if meta.is_file() {
                push_unique(&mut hits, candidate);
            }
        }
    }
    if let Ok(entries) = fs::read_dir(home.join(".config")) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if skip_config_entry(&file_name) {
                continue;
            }
            if config_name_variants(name)
                .iter()
                .any(|variant| file_name.eq_ignore_ascii_case(variant))
            {
                let path = entry.path();
                if path.is_dir() {
                    collect_config_dir(&path, &mut hits);
                } else {
                    push_unique(&mut hits, path);
                }
            }
        }
    }
    hits.sort();
    hits.dedup();
    Ok(hits)
}

pub fn locate(query: &str) -> io::Result<Vec<PathBuf>> {
    let mut hits = find_files(query)?;
    for path in find_config(query)? {
        push_unique(&mut hits, path);
    }
    for path in find_logs(query)? {
        push_unique(&mut hits, path);
    }
    if hits.is_empty() {
        for path in spotlight(query, 20)? {
            push_unique(&mut hits, path);
        }
    }
    Ok(hits)
}

pub fn search_files(
    root: &Path,
    name_contains: Option<&str>,
    extension: Option<&str>,
    max_entries: usize,
    max_results: usize,
) -> io::Result<Vec<PathBuf>> {
    let name_needle = name_contains.map(|value| value.trim().to_lowercase());
    let extension_needle =
        extension.map(|value| value.trim().trim_start_matches('.').to_lowercase());
    if name_needle.as_deref().is_none_or(str::is_empty)
        && extension_needle.as_deref().is_none_or(str::is_empty)
    {
        return Err(io::Error::other("file search requires a name or extension"));
    }
    let mut hits = Vec::new();
    let mut seen = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        seen += 1;
        if seen > max_entries {
            return Err(io::Error::other(format!(
                "file search limit of {max_entries} entries reached"
            )));
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_file() {
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            let name_matches = name_needle
                .as_deref()
                .is_none_or(|needle| file_name.to_lowercase().contains(needle));
            let extension_matches = extension_needle.as_deref().is_none_or(|needle| {
                path.extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(needle))
            });
            if name_matches && extension_matches {
                hits.push(path);
                if hits.len() >= max_results {
                    break;
                }
            }
        } else if metadata.is_dir() && (path == root || !skip_search_path(&path)) {
            for entry in fs::read_dir(path)? {
                stack.push(entry?.path());
            }
        }
    }
    hits.sort();
    Ok(hits)
}

fn skip_search_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(
                "node_modules"
                    | ".Trash"
                    | ".git"
                    | ".venv"
                    | "venv"
                    | "target"
                    | ".next"
                    | "dist"
                    | "build"
                    | "coverage"
                    | "Caches"
                    | "Cache"
            )
        )
    })
}

pub fn find_logs(name: &str) -> io::Result<Vec<PathBuf>> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') {
        return Ok(Vec::new());
    }
    let home = home()?;
    let mut hits = Vec::new();
    let logs = home.join("Library/Logs");
    let named = logs.join(name);
    if named.is_dir() {
        collect_config_dir(&named, &mut hits);
    } else if named.is_file() {
        push_unique(&mut hits, named);
    }
    let direct = logs.join(format!("{name}.log"));
    if direct.is_file() {
        push_unique(&mut hits, direct);
    }
    if let Ok(entries) = fs::read_dir(&logs) {
        for entry in entries.flatten().take(200) {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.to_lowercase().contains(&name.to_lowercase()) {
                let path = entry.path();
                if path.is_dir() {
                    collect_config_dir(&path, &mut hits);
                } else {
                    push_unique(&mut hits, path);
                }
            }
        }
    }
    for extra in [
        PathBuf::from("/opt/homebrew/var/log").join(name),
        home.join(".config").join(name).join("logs"),
        home.join(".config").join(name).join("log"),
    ] {
        if extra.is_dir() {
            collect_config_dir(&extra, &mut hits);
        } else if extra.is_file() {
            push_unique(&mut hits, extra);
        }
    }
    Ok(hits)
}

pub fn spotlight(query: &str, limit: usize) -> io::Result<Vec<PathBuf>> {
    let query = query.trim();
    if query.len() < 3 && !query.contains('.') {
        return Ok(Vec::new());
    }
    if query.contains(['\0', '"', '\n']) {
        return Ok(Vec::new());
    }
    let home = home()?;
    let Some(root) = home.to_str() else {
        return Ok(Vec::new());
    };
    let output = match Command::new("mdfind")
        .args(["-onlyin", root, "-name", query])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut hits = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.is_empty() {
            continue;
        }
        let path = PathBuf::from(line);
        if skip_search_path(&path) {
            continue;
        }
        push_unique(&mut hits, path);
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

pub fn preferred_config(paths: &[PathBuf], name: &str) -> Option<PathBuf> {
    let name = name.to_lowercase();
    let mut files: Vec<&PathBuf> = paths.iter().filter(|path| path.is_file()).collect();
    if files.is_empty() {
        return paths.first().cloned();
    }
    files.sort_by_key(|path| {
        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let score = if file.contains(&name) && looks_like_config_file(&file) {
            0u8
        } else if looks_like_config_file(&file) {
            1
        } else {
            2
        };
        (score, file)
    });
    files.into_iter().next().cloned()
}

pub fn size(path: &Path, max_entries: usize) -> io::Result<u64> {
    let top = path.display().to_string();
    let mut total = 0u64;
    let mut seen = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(path) = stack.pop() {
        seen += 1;
        if seen > max_entries {
            return Err(io::Error::other(format!(
                "directory scan limit of {max_entries} entries reached while measuring {top}"
            )));
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        } else if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                stack.push(entry?.path());
            }
        }
    }
    Ok(total)
}

pub fn large_files(
    path: &Path,
    minimum: u64,
    max_entries: usize,
    max_results: usize,
    file_type: Option<FileType>,
    max_age_days: Option<u32>,
) -> io::Result<Vec<(u64, PathBuf)>> {
    let modified_after = max_age_days.and_then(|days| {
        SystemTime::now().checked_sub(Duration::from_secs(u64::from(days) * 86_400))
    });
    let mut hits = Vec::new();
    let mut seen = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(path) = stack.pop() {
        seen += 1;
        if seen > max_entries {
            return Err(io::Error::other("directory scan limit reached"));
        }
        let metadata = fs::symlink_metadata(&path)?;
        let type_matches = file_type.is_none_or(|kind| {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            match kind {
                FileType::Video => ["mp4", "mov", "mkv", "avi", "webm", "m4v", "wmv"]
                    .iter()
                    .any(|value| extension.eq_ignore_ascii_case(value)),
                FileType::Pdf => extension.eq_ignore_ascii_case("pdf"),
                FileType::Image => ["jpg", "jpeg", "png", "heic", "gif", "webp", "tif", "tiff"]
                    .iter()
                    .any(|value| extension.eq_ignore_ascii_case(value)),
            }
        });
        let age_matches = modified_after
            .is_none_or(|cutoff| metadata.modified().is_ok_and(|modified| modified >= cutoff));
        if metadata.is_file() && metadata.len() >= minimum && type_matches && age_matches {
            hits.push((metadata.len(), path));
            hits.sort_by_key(|a| std::cmp::Reverse(a.0));
            hits.truncate(max_results);
        } else if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                stack.push(entry?.path());
            }
        }
    }
    Ok(hits)
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub command: String,
    pub user: String,
}

pub fn port_processes(port: u16) -> io::Result<Vec<Process>> {
    let arg = format!("-iTCP:{port}");
    let out = command("lsof", &["-nP", "-Fpcu", &arg, "-sTCP:LISTEN"])?;
    if !out.status.success() && out.stdout.is_empty() {
        return Ok(Vec::new());
    }
    Ok(parse_lsof_fields(&String::from_utf8_lossy(&out.stdout)))
}

fn parse_lsof_fields(value: &str) -> Vec<Process> {
    let mut found = Vec::new();
    let mut current: Option<Process> = None;
    for line in value.lines() {
        match line.split_at(1) {
            ("p", value) => {
                if let Some(process) = current.take() {
                    found.push(process);
                }
                current = value.parse().ok().map(|pid| Process {
                    pid,
                    command: String::new(),
                    user: String::new(),
                });
            }
            ("c", value) => {
                if let Some(p) = &mut current {
                    p.command = value.to_string();
                }
            }
            ("u", value) => {
                if let Some(p) = &mut current {
                    p.user = value.to_string();
                }
            }
            _ => {}
        }
    }
    if let Some(process) = current {
        found.push(process);
    }
    found.sort_by_key(|p| p.pid);
    found.dedup_by_key(|p| p.pid);
    found
}

pub fn safe_pid(pid: u32) -> bool {
    pid > 1 && pid != std::process::id() && Some(pid) != parent_pid()
}

pub fn process_by_pid(pid: u32) -> io::Result<Option<String>> {
    let output = command("ps", &["-p", &pid.to_string(), "-o", "comm="])?;
    if !output.status.success() {
        return Ok(None);
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!name.is_empty()).then_some(name))
}

#[derive(Debug, PartialEq)]
struct ProcessUsage {
    pid: u32,
    cpu: f32,
    memory_percent: f32,
    rss_kib: u64,
    command: String,
}

fn parse_process_usage(value: &str) -> Vec<ProcessUsage> {
    value
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let cpu = fields.next()?.parse::<f32>().ok()?;
            let memory_percent = fields.next()?.parse::<f32>().ok()?;
            let rss_kib = fields.next()?.parse().ok()?;
            let command = fields.collect::<Vec<_>>().join(" ");
            if command.is_empty() || !cpu.is_finite() || !memory_percent.is_finite() {
                return None;
            }
            Some(ProcessUsage {
                pid,
                cpu,
                memory_percent,
                rss_kib,
                command,
            })
        })
        .collect()
}

pub fn process_list(sort: Option<&str>, limit: usize) -> io::Result<String> {
    let output = command("ps", &["-axo", "pid=,pcpu=,pmem=,rss=,comm="])?;
    if !output.status.success() {
        return Err(io::Error::other("ps could not list running processes"));
    }
    let mut processes = parse_process_usage(&String::from_utf8_lossy(&output.stdout));
    processes.retain(|process| {
        let name = Path::new(&process.command)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&process.command);
        !matches!(name, "ps" | "(ps)")
    });
    if sort == Some("browser") {
        processes.retain(|process| {
            let command = process.command.to_lowercase();
            [
                "safari", "chrome", "chromium", "firefox", "edge", "brave", "opera", "vivaldi",
                "arc",
            ]
            .iter()
            .any(|browser| command.contains(browser))
        });
    }
    match sort {
        Some("memory") => processes.sort_by(|left, right| {
            right
                .rss_kib
                .cmp(&left.rss_kib)
                .then_with(|| right.memory_percent.total_cmp(&left.memory_percent))
        }),
        _ => processes.sort_by(|left, right| right.cpu.total_cmp(&left.cpu)),
    }
    let heading = match sort {
        Some("memory") => "Top processes by memory",
        Some("browser") => "Running browser processes",
        _ => "Top processes by CPU",
    };
    let rows = processes
        .into_iter()
        .take(limit.clamp(1, 100))
        .map(|process| {
            let label = Path::new(&process.command)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&process.command);
            format!(
                "{:>6}  {:>6.1}%  {:>6.1}%  {:>8.1} MiB  {}",
                process.pid,
                process.cpu,
                process.memory_percent,
                process.rss_kib as f64 / 1024.0,
                label
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return Err(io::Error::other("ps returned no process rows"));
    }
    Ok(format!(
        "{heading}\n   PID     CPU      MEM       RSS  COMMAND\n{}",
        rows.join("\n")
    ))
}

fn parent_pid() -> Option<u32> {
    unsafe extern "C" {
        fn getppid() -> i32;
    }
    u32::try_from(unsafe { getppid() }).ok()
}

pub fn executable(name: &str) -> Option<PathBuf> {
    if name.contains('/') || name.contains('\0') {
        return None;
    }
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn checked_text(program: &str, args: &[&str]) -> io::Result<String> {
    let output = command(program, args)?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if detail.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            format!("{program}: {detail}")
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn system_info() -> io::Result<String> {
    let version = checked_text("sw_vers", &[])?;
    let architecture = checked_text("uname", &["-m"])?;
    let kernel = checked_text("uname", &["-r"])?;
    let hostname = checked_text("hostname", &[])?;
    let pretty_version = version
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| format!("{}: {}", key.trim(), value.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "{pretty_version}\nArchitecture: {architecture}\nKernel: {kernel}\nHostname: {hostname}"
    ))
}

pub fn uptime_info() -> io::Result<String> {
    checked_text("uptime", &[])
}

pub fn human_bytes(bytes: u64) -> String {
    const GIB: f64 = 1_073_741_824.0;
    const MIB: f64 = 1_048_576.0;
    const KIB: f64 = 1024.0;
    if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / GIB)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn vm_value(vm_stat: &str, label: &str, page_size: u64) -> Option<u64> {
    let pages = vm_stat
        .lines()
        .find(|line| line.starts_with(label))?
        .split_once(':')?
        .1
        .trim()
        .trim_end_matches('.')
        .parse::<u64>()
        .ok()?;
    pages.checked_mul(page_size)
}

pub fn memory_info() -> io::Result<String> {
    let total = checked_text("sysctl", &["-n", "hw.memsize"])?
        .parse::<u64>()
        .map_err(|_| io::Error::other("sysctl returned an invalid memory size"))?;
    let vm = checked_text("vm_stat", &[])?;
    let page_size = vm
        .lines()
        .next()
        .and_then(|line| line.split("page size of ").nth(1))
        .and_then(|tail| tail.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(4096);
    let fields = [
        ("Free", "Pages free"),
        ("Active", "Pages active"),
        ("Inactive", "Pages inactive"),
        ("Wired", "Pages wired down"),
        ("Compressed", "Pages occupied by compressor"),
    ];
    let details = fields
        .into_iter()
        .filter_map(|(name, label)| {
            vm_value(&vm, label, page_size).map(|bytes| format!("{name}: {}", human_bytes(bytes)))
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "Physical memory: {}\n{details}",
        human_bytes(total)
    ))
}

pub fn cpu_info() -> io::Result<String> {
    let model = checked_text("sysctl", &["-n", "machdep.cpu.brand_string"])
        .or_else(|_| checked_text("sysctl", &["-n", "hw.model"]))?;
    let physical = checked_text("sysctl", &["-n", "hw.physicalcpu"])?;
    let logical = checked_text("sysctl", &["-n", "hw.logicalcpu"])?;
    Ok(format!(
        "Processor: {model}\nPhysical cores: {physical}\nLogical cores: {logical}"
    ))
}

pub fn network_info() -> io::Result<String> {
    let route = checked_text("route", &["-n", "get", "default"]).unwrap_or_default();
    let default_interface = route.lines().find_map(|line| {
        line.trim()
            .strip_prefix("interface:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
    });
    let interfaces = checked_text("ifconfig", &[])?;
    let mut current = "";
    let mut addresses = Vec::new();
    for line in interfaces.lines() {
        if !line.starts_with([' ', '\t']) {
            current = line.split(':').next().unwrap_or("");
            continue;
        }
        let trimmed = line.trim();
        if let Some(address) = trimmed
            .strip_prefix("inet ")
            .and_then(|rest| rest.split_whitespace().next())
            && address != "127.0.0.1"
        {
            addresses.push(format!("{current}: {address}"));
        }
    }
    addresses.sort();
    addresses.dedup();
    let default = default_interface
        .map(|name| format!("Default interface: {name}\n"))
        .unwrap_or_default();
    let body = if addresses.is_empty() {
        "No active non-loopback IPv4 address found.".to_string()
    } else {
        addresses.join("\n")
    };
    Ok(format!("{default}{body}"))
}

#[derive(Debug, Clone, PartialEq)]
struct PingStats {
    transmitted: u32,
    received: u32,
    loss_percent: f32,
    average_ms: Option<f32>,
}

fn parse_ping_stats(output: &str) -> Option<PingStats> {
    let packets = output
        .lines()
        .find(|line| line.contains("packets transmitted") && line.contains("packet loss"))?;
    let parts = packets.split(',').map(str::trim).collect::<Vec<_>>();
    let transmitted = parts.first()?.split_whitespace().next()?.parse().ok()?;
    let received = parts.get(1)?.split_whitespace().next()?.parse().ok()?;
    let loss_percent = parts
        .iter()
        .find(|part| part.contains("packet loss"))?
        .split('%')
        .next()?
        .split_whitespace()
        .next_back()?
        .parse()
        .ok()?;
    let average_ms = output
        .lines()
        .find(|line| line.contains("min/avg/max"))
        .and_then(|line| line.split('=').nth(1))
        .and_then(|values| values.trim().split('/').nth(1))
        .and_then(|value| value.parse().ok());
    Some(PingStats {
        transmitted,
        received,
        loss_percent,
        average_ms,
    })
}

fn ping_target(label: &str, host: &str) -> (String, Option<PingStats>) {
    let output = command(
        "/sbin/ping",
        &["-q", "-c", "4", "-i", "0.25", "-W", "1000", host],
    );
    let Ok(output) = output else {
        return (format!("{label} ({host}): ping unavailable"), None);
    };
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let stats = parse_ping_stats(&text);
    let summary = match &stats {
        Some(stats) => {
            let latency = stats
                .average_ms
                .map(|value| format!(", avg {value:.1} ms"))
                .unwrap_or_default();
            format!(
                "{label} ({host}): {:.1}% loss, {}/{} replies{latency}",
                stats.loss_percent, stats.received, stats.transmitted
            )
        }
        None => format!("{label} ({host}): no usable ping response"),
    };
    (summary, stats)
}

fn route_field(route: &str, name: &str) -> Option<String> {
    route.lines().find_map(|line| {
        line.trim()
            .strip_prefix(name)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn dns_servers(output: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut servers = Vec::new();
    for server in output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nameserver["))
        .filter_map(|line| line.split_once(':').map(|(_, value)| value.trim()))
        .filter(|value| !value.is_empty())
    {
        if seen.insert(server.to_string()) {
            servers.push(server.to_string());
        }
        if servers.len() >= 8 {
            break;
        }
    }
    servers
}

fn active_vpns(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.contains("(Connected)"))
        .map(|line| {
            line.split('"')
                .nth(1)
                .map(str::to_string)
                .unwrap_or_else(|| line.trim().to_string())
        })
        .collect()
}

fn enabled_proxies(output: &str) -> Vec<&'static str> {
    [
        ("HTTPEnable", "HTTP"),
        ("HTTPSEnable", "HTTPS"),
        ("SOCKSEnable", "SOCKS"),
        ("ProxyAutoConfigEnable", "automatic configuration"),
        ("ProxyAutoDiscoveryEnable", "automatic discovery"),
    ]
    .into_iter()
    .filter_map(|(key, label)| {
        output
            .lines()
            .any(|line| line.trim() == format!("{key} : 1"))
            .then_some(label)
    })
    .collect()
}

fn listening_tcp_ports(limit: usize) -> Vec<String> {
    let Ok(output) = command("/usr/sbin/lsof", &["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"]) else {
        return Vec::new();
    };
    let mut pid = String::new();
    let mut process = String::new();
    let mut listeners = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((field, value)) = line.split_at_checked(1) else {
            continue;
        };
        match field {
            "p" => pid = value.to_string(),
            "c" => process = value.to_string(),
            "n" if !value.is_empty() => {
                listeners.push(format!("{value} · {process} · PID {pid}"));
                if listeners.len() >= limit {
                    break;
                }
            }
            _ => {}
        }
    }
    listeners.sort();
    listeners.dedup();
    listeners
}

pub fn network_diagnosis() -> io::Result<String> {
    let route = checked_text("/sbin/route", &["-n", "get", "default"]).unwrap_or_default();
    let default_interface = route_field(&route, "interface:").unwrap_or_else(|| "unknown".into());
    let gateway = route_field(&route, "gateway:");

    let wifi = wifi_device().unwrap_or_else(|_| "unknown".into());
    let wifi_text = if wifi == "unknown" {
        "Wi-Fi: hardware interface not found".to_string()
    } else {
        let details = checked_text("/sbin/ifconfig", &[&wifi]).unwrap_or_default();
        let active = details.lines().any(|line| line.trim() == "status: active");
        let address = details.lines().find_map(|line| {
            line.trim()
                .strip_prefix("inet ")
                .and_then(|rest| rest.split_whitespace().next())
        });
        format!(
            "Wi-Fi: {wifi} · {}{}",
            if active { "active" } else { "inactive" },
            address
                .map(|value| format!(" · {value}"))
                .unwrap_or_default()
        )
    };

    let dns = checked_text("/usr/sbin/scutil", &["--dns"]).unwrap_or_default();
    let servers = dns_servers(&dns);
    let lookup_started = Instant::now();
    let lookup = command(
        "/usr/bin/dscacheutil",
        &["-q", "host", "-a", "name", "example.com"],
    );
    let lookup_ms = lookup_started.elapsed().as_millis();
    let lookup_ok = lookup.is_ok_and(|output| output.status.success() && !output.stdout.is_empty());

    let physical_gateway = if wifi == "unknown" {
        None
    } else {
        checked_text("/usr/sbin/ipconfig", &["getoption", &wifi, "router"]).ok()
    };
    let gateway_target = physical_gateway.as_deref().or(gateway.as_deref());
    let (gateway_ping, gateway_stats) = gateway_target
        .map(|host| ping_target("Gateway", host))
        .unwrap_or_else(|| ("Gateway: not found".into(), None));
    let (internet_ping, internet_stats) = ping_target("Internet", "1.1.1.1");

    let vpn_output = checked_text("/usr/sbin/scutil", &["--nc", "list"]).unwrap_or_default();
    let vpns = active_vpns(&vpn_output);
    let proxy_output = checked_text("/usr/sbin/scutil", &["--proxy"]).unwrap_or_default();
    let proxies = enabled_proxies(&proxy_output);
    let listeners = listening_tcp_ports(20);

    let mut findings = Vec::new();
    if default_interface.starts_with("utun") {
        findings.push(format!(
            "Default traffic is routed through tunnel interface {default_interface}."
        ));
    }
    if !vpns.is_empty() {
        findings.push(format!(
            "{} VPN connection(s) are active; compare performance with them paused.",
            vpns.len()
        ));
    }
    if !proxies.is_empty() {
        findings.push(format!(
            "Active proxy settings can add latency: {}.",
            proxies.join(", ")
        ));
    }
    if servers.is_empty() || !lookup_ok {
        findings.push("DNS resolution failed or no resolver was reported.".into());
    } else if lookup_ms > 250 {
        findings.push(format!("DNS lookup is slow ({lookup_ms} ms)."));
    }
    for (label, stats) in [("gateway", gateway_stats), ("internet", internet_stats)] {
        if let Some(stats) = stats {
            if stats.loss_percent > 0.0 {
                findings.push(format!(
                    "ICMP packet loss to {label} is {:.1}% (some VPNs block ping).",
                    stats.loss_percent
                ));
            }
            if stats.average_ms.is_some_and(|latency| latency > 100.0) {
                findings.push(format!("Latency to {label} is high."));
            }
        } else {
            findings.push(format!("Could not measure packet loss to {label}."));
        }
    }
    if findings.is_empty() {
        findings
            .push("No obvious DNS, routing, proxy, packet-loss, or latency issue found.".into());
    }

    let route_line = match gateway {
        Some(gateway) => format!("Route: {default_interface} via {gateway}"),
        None => format!("Route: {default_interface}"),
    };
    let dns_line = if servers.is_empty() {
        format!("DNS: no resolvers found · example.com lookup {lookup_ms} ms · failed")
    } else {
        format!(
            "DNS: {} · example.com lookup {lookup_ms} ms · {}",
            servers.join(", "),
            if lookup_ok { "ok" } else { "failed" }
        )
    };
    let vpn_line = if vpns.is_empty() {
        "VPN: none connected".to_string()
    } else {
        format!("VPN: {}", vpns.join(", "))
    };
    let proxy_line = if proxies.is_empty() {
        "Proxy: off".to_string()
    } else {
        format!("Proxy: {}", proxies.join(", "))
    };
    let ports = if listeners.is_empty() {
        "Listening TCP ports: none visible".to_string()
    } else {
        format!(
            "Listening TCP ports (showing {}):\n{}",
            listeners.len(),
            listeners
                .iter()
                .map(|listener| format!("- {listener}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let findings = findings
        .iter()
        .map(|finding| format!("- {finding}"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "Network diagnosis\n{route_line}\n{wifi_text}\n{dns_line}\n{gateway_ping}\n{internet_ping}\n{vpn_line}\n{proxy_line}\n\n{ports}\n\nFindings:\n{findings}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_guard() {
        assert!(protected(Path::new("/")));
        assert!(protected(Path::new("/Applications/..")));
        assert!(!protected(Path::new("/tmp/terfinder-test")));
    }
    #[test]
    fn finder_fallback_is_limited_to_application_bundles() {
        assert!(finder_can_trash(Path::new("/Applications/Zoom.app")));
        assert!(finder_can_trash(Path::new(
            "/Applications/Utilities/Example.app"
        )));
        assert!(!finder_can_trash(Path::new("/Applications/notes.txt")));
        assert!(!finder_can_trash(Path::new("/tmp/Zoom.app")));
    }
    #[test]
    fn app_leftovers_require_a_specific_identity_match() {
        let terms = vec!["google".to_string(), "chrome".to_string()];
        assert!(matches_app_leftover(
            "com.google.Chrome.plist",
            "Google Chrome",
            Some("com.google.Chrome"),
            &terms
        ));
        assert!(!matches_app_leftover(
            "com.google.Drive.plist",
            "Google Chrome",
            Some("com.google.Chrome"),
            &terms
        ));
        let zoom = vec!["zoom".to_string()];
        assert!(matches_app_leftover(
            "ZoomUpdater",
            "zoom.us",
            Some("us.zoom.xos"),
            &zoom
        ));
        assert!(matches_app_leftover(
            "us.zoom.xos.binarycookies",
            "zoom.us",
            Some("us.zoom.xos"),
            &zoom
        ));
        assert!(!matches_app_leftover(
            "unrelated.plist",
            "zoom.us",
            Some("us.zoom.xos"),
            &zoom
        ));
    }
    #[test]
    fn network_diagnostic_parsers_extract_signal() {
        let ping = "4 packets transmitted, 3 packets received, 25.0% packet loss\n\
                    round-trip min/avg/max/stddev = 10.000/20.500/30.000/2.000 ms\n";
        assert_eq!(
            parse_ping_stats(ping),
            Some(PingStats {
                transmitted: 4,
                received: 3,
                loss_percent: 25.0,
                average_ms: Some(20.5),
            })
        );
        assert_eq!(
            dns_servers("resolver #1\n  nameserver[0] : 1.1.1.1\n  nameserver[1] : 8.8.8.8\n"),
            vec!["1.1.1.1", "8.8.8.8"]
        );
        assert_eq!(
            active_vpns("* (Connected) 123 VPN (com.example) \"Work VPN\" [VPN:com.example]\n"),
            vec!["Work VPN"]
        );
        assert_eq!(
            enabled_proxies("<dictionary> {\n  HTTPEnable : 1\n  HTTPSEnable : 0\n}"),
            vec!["HTTP"]
        );
    }
    #[test]
    fn pid_guard() {
        assert!(!safe_pid(0));
        assert!(!safe_pid(1));
        assert!(!safe_pid(std::process::id()));
    }
    #[test]
    fn parse_port_listener() {
        let found = parse_lsof_fields("p123\ncPython\nu501\np123\ncPython\nu501\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 123);
        assert_eq!(found[0].command, "Python");
    }
    #[test]
    fn process_usage_is_parsed_and_ranked_data_is_available() {
        let rows = parse_process_usage(
            "  12  8.5  1.2  2048 /usr/bin/alpha\n  42  0.1  4.8 8192 /Applications/Beta App\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 12);
        assert_eq!(rows[1].rss_kib, 8192);
        assert_eq!(rows[1].command, "/Applications/Beta App");
    }
    #[test]
    fn formats_vm_values() {
        let vm = "Mach Virtual Memory Statistics: (page size of 4096 bytes)\nPages free: 256.\n";
        assert_eq!(vm_value(vm, "Pages free", 4096), Some(1_048_576));
        assert_eq!(human_bytes(1_073_741_824), "1.0 GiB");
    }
    #[test]
    fn parses_wifi_hardware_port() {
        let ports = "Hardware Port: Ethernet\nDevice: en1\n\
                     Ethernet Address: aa:bb:cc:dd:ee:ff\n\n\
                     Hardware Port: Wi-Fi\nDevice: en0\n\
                     Ethernet Address: 11:22:33:44:55:66\n";
        assert_eq!(parse_wifi_device(ports).as_deref(), Some("en0"));
        assert_eq!(
            parse_wifi_device("Hardware Port: Ethernet\nDevice: en1\n"),
            None
        );
    }
    #[test]
    fn rename_never_overwrites_existing_file() {
        let dir = env::temp_dir().join(format!("terfinder-rename-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.txt");
        let destination = dir.join("destination.txt");
        fs::write(&source, b"source").unwrap();
        fs::write(&destination, b"destination").unwrap();
        assert_eq!(
            rename_no_replace(&source, &destination).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&source).unwrap(), b"source");
        assert_eq!(fs::read(&destination).unwrap(), b"destination");
        fs::remove_file(source).unwrap();
        fs::remove_file(destination).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn copy_never_overwrites_existing_file() {
        let dir = env::temp_dir().join(format!("terfinder-copy-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.txt");
        let destination = dir.join("destination.txt");
        fs::write(&source, b"source").unwrap();
        fs::write(&destination, b"destination").unwrap();
        assert_eq!(
            copy_no_replace(&source, &destination).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&destination).unwrap(), b"destination");
        fs::remove_file(&destination).unwrap();
        copy_no_replace(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"source");
        fs::remove_file(source).unwrap();
        fs::remove_file(destination).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn finds_config_for_any_tool_name() {
        let dir = env::temp_dir().join(format!("terfinder-config-test-{}", std::process::id()));
        fs::create_dir_all(dir.join(".config/acme")).unwrap();
        fs::write(dir.join(".config/acme/acme.json"), b"{}").unwrap();
        fs::write(dir.join(".config/acme/package-lock.json"), b"{}").unwrap();
        fs::create_dir_all(dir.join(".config/acme/node_modules")).unwrap();
        let hits = find_config_in(&dir, "acme").unwrap();
        assert!(
            hits.iter()
                .any(|path| path.file_name().unwrap() == "acme.json"),
            "{hits:?}"
        );
        assert!(hits.iter().any(|path| path.ends_with("acme")), "{hits:?}");
        assert!(
            hits.iter()
                .all(|path| path.file_name().unwrap() != "package-lock.json"),
            "{hits:?}"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn large_files_respects_type_age_and_limit() {
        let dir = env::temp_dir().join(format!(
            "terfinder-large-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("recent.MP4"), vec![0; 10]).unwrap();
        fs::write(dir.join("old.mov"), vec![0; 20]).unwrap();
        fs::write(dir.join("document.pdf"), vec![0; 30]).unwrap();
        let old = fs::File::options()
            .write(true)
            .open(dir.join("old.mov"))
            .unwrap();
        old.set_modified(SystemTime::now() - Duration::from_secs(40 * 86_400))
            .unwrap();
        let hits = large_files(&dir, 0, 100, 10, Some(FileType::Video), Some(30)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1.file_name().unwrap(), "recent.MP4");
        assert!(
            large_files(&dir, 11, 100, 10, Some(FileType::Video), Some(30))
                .unwrap()
                .is_empty()
        );
        assert_eq!(large_files(&dir, 0, 100, 1, None, None).unwrap().len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recursive_file_search_filters_and_skips_build_directories() {
        let dir =
            env::temp_dir().join(format!("terfinder-file-search-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("src/nested")).unwrap();
        fs::create_dir_all(dir.join("target/debug")).unwrap();
        fs::write(dir.join("src/nested/data.JSON"), b"{}").unwrap();
        fs::write(dir.join("src/README.md"), b"readme").unwrap();
        fs::write(dir.join("target/debug/ignored.json"), b"{}").unwrap();

        let json = search_files(&dir, None, Some("json"), 100, 10).unwrap();
        assert_eq!(json.len(), 1);
        assert_eq!(json[0].file_name().unwrap(), "data.JSON");
        let readme = search_files(&dir, Some("readme"), None, 100, 10).unwrap();
        assert_eq!(readme.len(), 1);
        assert_eq!(readme[0].file_name().unwrap(), "README.md");

        fs::remove_dir_all(dir).unwrap();
    }
}
