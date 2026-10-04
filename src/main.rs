mod alias;
mod commands;
mod config;
mod fast;
mod frame;
mod intent;
mod knowledge;
mod laya;
mod lexicon;
mod system;
mod tools;

use intent::{Action, Intent, Risk, capability};
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

#[derive(Debug)]
enum Plan {
    Read(String),
    Open(PathBuf),
    OpenUrl(String),
    Trash(PathBuf),
    RemoveAppCompletely(system::AppCleanup),
    Move(PathBuf, PathBuf),
    Terminate {
        pid: u32,
        process: Option<String>,
        port: Option<u16>,
    },
    Quit {
        label: String,
        processes: Vec<(u32, String)>,
    },
    Cache(String),
    Bluetooth(bool),
    AirDrop {
        mode: String,
        enable_bluetooth: bool,
        enable_wifi: Option<String>,
    },
    StageManager(bool),
    FetchUrl(String),
    Copy(PathBuf, PathBuf),
    CreateDir(PathBuf),
    ChangeDir(PathBuf),
    Tool {
        program: String,
        args: Vec<String>,
    },
    UpdateCli {
        label: String,
        executable: PathBuf,
        before_version: String,
    },
}

fn help() {
    println!(
        "TerFinder {} — скажи, что нужно; синтаксис учить не надо.\n\n  tf почему мак тормозит\n  tf кому принадлежит PID 46272\n  tf покажи, кто занял порт 3000\n  tf найди все .tsx файлы в этом проекте\n  tf what changed in this git repository\n  tf turn Bluetooth off\n  tf включи AirDrop\n\nРусский, English и ещё 15 языков можно смешивать. Обычные запросы разбираются мгновенно; непривычные формулировки понимает локальная модель. Она выбирает только из безопасного реестра типизированных действий и никогда не генерирует shell-команды.\n\nИзменения всегда показываются заранее и требуют подтверждения. Неуверенный запрос ничего не запускает.\n\n  tf                      диалоговый режим\n  tf plan <запрос>        только показать план\n  tf doctor               проверить установку\n  tf help-advanced        служебные команды",
        env!("CARGO_PKG_VERSION")
    );
}

fn advanced_help() {
    println!(
        "TerFinder {} — advanced\n\nUsage: tf [--dry-run] [--yes] <request>\n       tf                       interactive mode\n       tf plan <request>        inspect without executing\n       tf capabilities [--json]\n       tf remember <phrase> = <known request>\n       tf misses\n       tf knowledge\n       tf model <start|stop|status>\n       tf daemon <start|stop|status>\n       tf doctor\n       tf setup\n\nOptions:\n  --dry-run         Resolve and show a plan without executing it\n  --yes             Approve ordinary destructive actions\n  --fast-only       Do not use semantic fallback\n  --interpret-only  Print typed intent JSON without inspecting targets\n\n`model` and `daemon` are aliases. A stopped model is normal: it starts on demand and exits after its idle timeout. Every accepted request maps to a typed action; no model-generated shell command is executed. --yes does not approve high-risk actions.",
        env!("CARGO_PKG_VERSION")
    );
}

fn choose(paths: Vec<PathBuf>, dry: bool) -> Result<PathBuf, String> {
    if paths.is_empty() {
        return Err("No matching target found.".into());
    }
    if paths.len() == 1 {
        return Ok(paths[0].clone());
    }
    println!("Multiple targets found:");
    for (n, path) in paths.iter().enumerate() {
        println!("[{}] {}", n + 1, path.display());
    }
    if dry {
        return Err("Ambiguous target; no changes made.".into());
    }
    print!("Choose [1-{}]: ", paths.len());
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let n: usize = line
        .trim()
        .parse()
        .map_err(|_| "No target selected.".to_string())?;
    paths
        .get(n.saturating_sub(1))
        .cloned()
        .ok_or_else(|| "Invalid selection.".into())
}

fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("Missing {name}."))
}

fn checked_http_url(value: &str) -> Result<String, String> {
    let url = lexicon::normalize_http_target(value)
        .ok_or("Only a valid HTTP or HTTPS URL is supported; no request was sent.")?;
    if url.len() > 2_048 {
        return Err("URL is longer than 2048 bytes; no request was sent.".into());
    }
    if url.chars().any(char::is_whitespace) || url.chars().any(char::is_control) {
        return Err("URL contains whitespace or control characters; no request was sent.".into());
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or("Only HTTP and HTTPS URLs are supported; no request was sent.")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err("URL must have a host and cannot contain embedded credentials.".into());
    }
    Ok(url)
}

fn plan(intent: &Intent, dry: bool) -> Result<(Risk, Plan), String> {
    let cap = capability(intent.action).ok_or("Unsupported action")?;
    let risk =
        if intent.action == Action::SetBluetoothPower && intent.target.as_deref() == Some("off") {
            Risk::Destructive
        } else if intent.action == Action::RunTool
            && intent.target.as_deref() == Some("cargo")
            && intent
                .argv
                .as_ref()
                .and_then(|args| args.first())
                .is_some_and(|sub| matches!(sub.as_str(), "test" | "build" | "clippy" | "fetch"))
        {
            Risk::Low
        } else {
            cap.risk
        };
    let plan = match intent.action {
        Action::FindPortProcess | Action::KillPortProcess => {
            let port = intent.port.ok_or("Missing port")?;
            let processes = system::port_processes(port).map_err(|e| e.to_string())?;
            if intent.action == Action::FindPortProcess {
                if processes.is_empty() {
                    Plan::Read(format!("Nothing is listening on port {port}."))
                } else {
                    Plan::Read(format!(
                        "Port {port}\n{}",
                        processes
                            .iter()
                            .map(|p| format!("PID {} · {} · UID {}", p.pid, p.command, p.user))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                }
            } else {
                if processes.len() != 1 {
                    return Err(format!(
                        "Expected one listener on port {port}, found {}. No changes made.",
                        processes.len()
                    ));
                }
                let p = &processes[0];
                if !system::safe_pid(p.pid) {
                    return Err("Protected PID; no changes made.".into());
                }
                Plan::Terminate {
                    pid: p.pid,
                    process: Some(p.command.clone()),
                    port: Some(port),
                }
            }
        }
        Action::FindApp
        | Action::RemoveApp
        | Action::RemoveAppCompletely
        | Action::OpenApp
        | Action::QuitApp => {
            let target = required(&intent.target, "app name")?;
            let paths = system::find_apps(target).map_err(|e| e.to_string())?;
            if intent.action == Action::FindApp {
                if paths.is_empty() {
                    return Err("No matching application found.".into());
                }
                return Ok((
                    risk,
                    Plan::Read(
                        paths
                            .iter()
                            .enumerate()
                            .map(|(i, path)| format!("[{}] {}", i + 1, path.display()))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                ));
            }
            if intent.action == Action::QuitApp {
                if paths.is_empty() {
                    let processes = system::running_processes(target);
                    if processes.is_empty() {
                        return Err(format!("No running application or process named {target}."));
                    }
                    return Ok((
                        risk,
                        Plan::Quit {
                            label: target.to_string(),
                            processes,
                        },
                    ));
                }
                let path = choose(paths, dry)?;
                let processes = system::app_processes(&path);
                if processes.is_empty() {
                    return Err(format!("{} is not running.", path.display()));
                }
                return Ok((
                    risk,
                    Plan::Quit {
                        label: path.display().to_string(),
                        processes,
                    },
                ));
            }
            if paths.is_empty() && intent.action == Action::OpenApp {
                if let Ok(path) = system::checked_existing(target) {
                    return Ok((risk, Plan::Open(path)));
                }
                if let Some(path) = system::existing_dir(target)
                    && !system::protected(&path)
                {
                    return Ok((risk, Plan::Open(path)));
                }
                if let Ok(files) = system::find_files(target)
                    && !files.is_empty()
                {
                    let path = choose(files, dry)?;
                    if !system::protected(&path) {
                        return Ok((risk, Plan::Open(path)));
                    }
                }
                return Err(format!(
                    "No application, folder or file named {target} was found."
                ));
            }
            let path = choose(paths, dry)?;
            match intent.action {
                Action::OpenApp => Plan::Open(path),
                Action::RemoveApp => {
                    if path.starts_with("/System") {
                        return Err("System applications cannot be removed.".into());
                    }
                    Plan::Trash(path)
                }
                Action::RemoveAppCompletely => {
                    if path.starts_with("/System") {
                        return Err("System applications cannot be removed.".into());
                    }
                    Plan::RemoveAppCompletely(
                        system::app_cleanup(&path).map_err(|error| error.to_string())?,
                    )
                }
                _ => unreachable!(),
            }
        }
        Action::UpdateApp => {
            let target = required(&intent.target, "app name")?;
            let normalized = target
                .to_lowercase()
                .chars()
                .filter(|character| character.is_alphanumeric())
                .collect::<String>();
            if !matches!(normalized.as_str(), "opencode" | "опенкод") {
                return Err(format!(
                    "No trusted updater is registered for {target}. No changes made."
                ));
            }
            let executable = system::executable("opencode")
                .ok_or("OpenCode is not installed or is not available on PATH.")?;
            let output = Command::new(&executable)
                .arg("--version")
                .output()
                .map_err(|error| format!("Could not inspect OpenCode: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "Could not inspect OpenCode version: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            let before_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if before_version.is_empty() {
                return Err("OpenCode returned an empty version; no changes made.".into());
            }
            Plan::UpdateCli {
                label: "OpenCode".into(),
                executable,
                before_version,
            }
        }
        Action::FindFile => {
            let query = required(&intent.target, "file name")?;
            let paths = system::locate(query).map_err(|e| e.to_string())?;
            Plan::Read(if paths.is_empty() {
                format!("No file or config found for {query}.")
            } else {
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        Action::FindFiles => {
            let target = required(&intent.target, "search directory")?;
            let path = system::normalize(&system::expand(target).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if !path.is_dir() {
                return Err(format!("{} is not a directory.", path.display()));
            }
            let hits = system::search_files(
                &path,
                intent.name_contains.as_deref(),
                intent.file_extension.as_deref(),
                200_000,
                intent.limit.unwrap_or(500).clamp(1, 1_000),
            )
            .map_err(|e| e.to_string())?;
            Plan::Read(if hits.is_empty() {
                "No matching files found.".into()
            } else {
                hits.iter()
                    .map(|item| item.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        Action::OpenFile => {
            let target = required(&intent.target, "file")?;
            if lexicon::looks_like_url(target) {
                let url = if target.contains("://") {
                    target.to_string()
                } else {
                    format!("https://{target}")
                };
                return Ok((risk, Plan::OpenUrl(url)));
            }
            if target.contains('/') || target.starts_with('~') {
                return match system::checked_existing(target) {
                    Ok(path) => Ok((risk, Plan::Open(path))),
                    Err(error) => Err(format!("Cannot open {}: {error}", target)),
                };
            }
            let mut paths = system::find_files(target).map_err(|e| e.to_string())?;
            if paths.is_empty() {
                paths = system::find_config(target).map_err(|e| e.to_string())?;
            }
            let path = choose(paths, dry)?;
            Plan::Open(path)
        }
        Action::DeleteFile | Action::DeleteDirectory => {
            let path = system::checked_existing(required(&intent.target, "path")?)
                .map_err(|e| e.to_string())?;
            if path.starts_with("/System") || path.starts_with("/Library") {
                return Err("System path is protected.".into());
            }
            Plan::Trash(path)
        }
        Action::MoveFile | Action::RenameFile => {
            let source = system::checked_existing(required(&intent.source, "source")?)
                .map_err(|e| e.to_string())?;
            let mut destination = system::normalize(
                &system::expand(required(&intent.destination, "destination")?)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if destination.is_dir() {
                destination.push(source.file_name().ok_or("Invalid source")?);
            }
            destination = system::canonical_parent(&destination).map_err(|e| e.to_string())?;
            if source.starts_with("/System")
                || source.starts_with("/Library")
                || destination.starts_with("/System")
                || destination.starts_with("/Library")
            {
                return Err("System path is protected.".into());
            }
            if destination.exists() || fs::symlink_metadata(&destination).is_ok() {
                return Err("Destination exists; no overwrite allowed.".into());
            }
            if destination.starts_with(&source) && source.is_dir() {
                return Err("Cannot move a directory inside itself.".into());
            }
            Plan::Move(source, destination)
        }
        Action::DirectorySize => {
            let target = required(&intent.target, "directory")?;
            let path = system::normalize(&system::expand(target).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let bytes = system::size(&path, 200_000).map_err(|e| e.to_string())?;
            Plan::Read(format!(
                "{}\n{} bytes ({:.2} GiB)",
                path.display(),
                bytes,
                bytes as f64 / 1_073_741_824.0
            ))
        }
        Action::FindLargeFiles => {
            let target = required(&intent.target, "directory")?;
            let path = system::normalize(&system::expand(target).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let minimum = intent.minimum_size_bytes.unwrap_or(0);
            let hits = system::large_files(
                &path,
                minimum,
                200_000,
                intent.limit.unwrap_or(30).min(100),
                intent.file_type,
                intent.max_age_days,
            )
            .map_err(|e| e.to_string())?;
            Plan::Read(if hits.is_empty() {
                "No matching files found.".into()
            } else {
                hits.iter()
                    .map(|(size, path)| format!("{size} bytes  {}", path.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        Action::FindProcess => {
            if let Some(pid) = intent.pid {
                return Ok((
                    risk,
                    Plan::Read(
                        match system::process_by_pid(pid).map_err(|e| e.to_string())? {
                            Some(process) => format!("PID {pid} · {process}"),
                            None => format!("No running process with PID {pid}."),
                        },
                    ),
                ));
            }
            let query = required(&intent.target, "process name")?;
            if query.contains('/') || query.contains(' ') {
                return Err("Process query must be a simple name.".into());
            }
            if let Some(path) = system::executable(query).or_else(|| {
                (query == "python")
                    .then(|| system::executable("python3"))
                    .flatten()
            }) {
                return Ok((risk, Plan::Read(path.display().to_string())));
            }
            let apps = system::find_apps(query).map_err(|e| e.to_string())?;
            if !apps.is_empty() {
                return Ok((
                    risk,
                    Plan::Read(
                        apps.iter()
                            .enumerate()
                            .map(|(i, path)| format!("[{}] {}", i + 1, path.display()))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                ));
            }
            let ps = system::command("ps", &["-axo", "pid=,comm="]).map_err(|e| e.to_string())?;
            let lines = String::from_utf8_lossy(&ps.stdout);
            let matches: Vec<_> = lines
                .lines()
                .filter(|line| line.to_lowercase().contains(&query.to_lowercase()))
                .take(30)
                .collect();
            if matches.is_empty() {
                Plan::Read(format!("No process or executable matching {query}."))
            } else {
                Plan::Read(matches.join("\n"))
            }
        }
        Action::KillProcess => {
            let pid = intent.pid.ok_or("Missing PID")?;
            if !system::safe_pid(pid) {
                return Err("Protected PID; no changes made.".into());
            }
            let process = system::process_by_pid(pid)
                .map_err(|e| e.to_string())?
                .ok_or("Process not found; no changes made.")?;
            Plan::Terminate {
                pid,
                process: Some(process),
                port: None,
            }
        }
        Action::ClearCache => {
            let cache = required(&intent.target, "cache name")?.to_lowercase();
            if !["pip", "npm", "yarn", "pnpm", "brew", "homebrew"].contains(&cache.as_str()) {
                return Err("Unsupported cache; no changes made.".into());
            }
            Plan::Cache(cache)
        }
        Action::ShowDiskUsage => {
            let output = system::command("df", &["-h", "/"]).map_err(|e| e.to_string())?;
            Plan::Read(String::from_utf8_lossy(&output.stdout).to_string())
        }
        Action::ShowBattery => {
            let output = system::command("pmset", &["-g", "batt"]).map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("Could not read battery status from pmset.".into());
            }
            Plan::Read(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Action::ListProcesses => {
            let output = if intent.target.as_deref() == Some("diagnostic") {
                format!(
                    "{}\n\n{}",
                    system::process_list(Some("cpu"), 12).map_err(|e| e.to_string())?,
                    system::process_list(Some("memory"), 12).map_err(|e| e.to_string())?
                )
            } else {
                system::process_list(intent.target.as_deref(), 20).map_err(|e| e.to_string())?
            };
            Plan::Read(output)
        }
        Action::SetBluetoothPower => {
            let power = required(&intent.target, "Bluetooth power")?;
            Plan::Bluetooth(match power {
                "on" => true,
                "off" => false,
                _ => return Err("Bluetooth power must be on or off.".into()),
            })
        }
        Action::SetAirDropMode => {
            let requested = required(&intent.target, "AirDrop mode")?;
            let current = system::airdrop_mode().map_err(|error| error.to_string())?;
            let mode = match requested {
                "off" => "Off".to_string(),
                "contacts" => "Contacts Only".to_string(),
                "everyone" => "Everyone".to_string(),
                "on" if current == "Off" => "Contacts Only".to_string(),
                "on" => current.clone(),
                _ => return Err("AirDrop mode must be on, off, contacts, or everyone.".into()),
            };
            if mode == "Off" && current == "Off" {
                return Ok((Risk::ReadOnly, Plan::Read("AirDrop is already off.".into())));
            }
            let (enable_bluetooth, enable_wifi) = if mode == "Off" {
                (false, None)
            } else {
                let enable_bluetooth =
                    !system::bluetooth_power().map_err(|error| error.to_string())?;
                let device = system::wifi_device().map_err(|error| error.to_string())?;
                let enable_wifi = (!system::wifi_power(&device)
                    .map_err(|error| error.to_string())?)
                .then_some(device);
                (enable_bluetooth, enable_wifi)
            };
            if mode == current && !enable_bluetooth && enable_wifi.is_none() {
                return Ok((
                    Risk::ReadOnly,
                    Plan::Read(format!("AirDrop is already on ({mode}).")),
                ));
            }
            Plan::AirDrop {
                mode,
                enable_bluetooth,
                enable_wifi,
            }
        }
        Action::SetStageManager => {
            let on = match required(&intent.target, "Stage Manager power")? {
                "on" => true,
                "off" => false,
                _ => return Err("Stage Manager power must be on or off.".into()),
            };
            if system::stage_manager_enabled().map_err(|error| error.to_string())? == on {
                return Ok((
                    Risk::ReadOnly,
                    Plan::Read(format!(
                        "Stage Manager is already {}.",
                        if on { "on" } else { "off" }
                    )),
                ));
            }
            Plan::StageManager(on)
        }
        Action::ListDirectory => {
            let path = match intent.target.as_deref() {
                Some(target) => {
                    system::normalize(&system::expand(target).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?
                }
                None => env::current_dir().map_err(|e| e.to_string())?,
            };
            if !path.is_dir() {
                return Err(format!("{} is not a directory.", path.display()));
            }
            Plan::Read(
                system::list_dir(&path, intent.hidden, intent.long).map_err(|e| e.to_string())?,
            )
        }
        Action::ReadFile => {
            let target = required(&intent.target, "file")?;
            let path = if target.contains('/') || target.starts_with('~') || target.contains('.') {
                system::checked_existing(target).map_err(|e| e.to_string())?
            } else {
                let mut paths = system::find_files(target).map_err(|e| e.to_string())?;
                if paths.is_empty() {
                    paths = system::find_config(target).map_err(|e| e.to_string())?;
                }
                system::preferred_config(&paths, target).ok_or_else(|| {
                    format!("No file found in bounded search locations for {target}.")
                })?
            };
            if path.is_dir() {
                return Err(format!("{} is a directory; use ls.", path.display()));
            }
            let max = intent.limit.unwrap_or(256_000).min(1_048_576);
            Plan::Read(system::read_text_file(&path, max).map_err(|e| e.to_string())?)
        }
        Action::CopyFile => {
            let source = system::checked_existing(required(&intent.source, "source")?)
                .map_err(|e| e.to_string())?;
            let mut destination = system::normalize(
                &system::expand(required(&intent.destination, "destination")?)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if destination.is_dir() {
                destination.push(source.file_name().ok_or("Invalid source")?);
            }
            destination = system::canonical_parent(&destination).map_err(|e| e.to_string())?;
            if source.starts_with("/System")
                || source.starts_with("/Library")
                || destination.starts_with("/System")
                || destination.starts_with("/Library")
            {
                return Err("System path is protected.".into());
            }
            if destination.exists() || fs::symlink_metadata(&destination).is_ok() {
                return Err("Destination exists; no overwrite allowed.".into());
            }
            Plan::Copy(source, destination)
        }
        Action::CreateDirectory => {
            let path = system::normalize(
                &system::expand(required(&intent.target, "directory")?)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if path.starts_with("/System")
                || path.starts_with("/Library")
                || system::protected(&path)
            {
                return Err("System path is protected.".into());
            }
            if path.exists() {
                return Err(format!("{} already exists.", path.display()));
            }
            Plan::CreateDir(path)
        }
        Action::PrintWorkingDirectory => Plan::Read(
            env::current_dir()
                .map_err(|e| e.to_string())?
                .display()
                .to_string(),
        ),
        Action::ChangeDirectory => {
            let target = required(&intent.target, "directory")?;
            let path = system::existing_dir(target)
                .or_else(|| {
                    system::normalize(&system::expand(target).ok()?)
                        .ok()
                        .filter(|path| path.is_dir())
                })
                .ok_or_else(|| format!("No such directory: {target}"))?;
            Plan::ChangeDir(path)
        }
        Action::ShowDate => {
            let output = system::command("date", &[]).map_err(|e| e.to_string())?;
            Plan::Read(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Action::WhoAmI => {
            let output = system::command("whoami", &[]).map_err(|e| e.to_string())?;
            Plan::Read(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Action::ShowHostname => {
            let output = system::command("hostname", &[]).map_err(|e| e.to_string())?;
            Plan::Read(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Action::ShowSystemInfo => Plan::Read(system::system_info().map_err(|e| e.to_string())?),
        Action::ShowUptime => Plan::Read(system::uptime_info().map_err(|e| e.to_string())?),
        Action::ShowMemory => Plan::Read(system::memory_info().map_err(|e| e.to_string())?),
        Action::ShowCpu => Plan::Read(system::cpu_info().map_err(|e| e.to_string())?),
        Action::ShowNetwork => Plan::Read(system::network_info().map_err(|e| e.to_string())?),
        Action::DiagnoseNetwork => {
            Plan::Read(system::network_diagnosis().map_err(|e| e.to_string())?)
        }
        Action::FetchUrl => {
            let url = checked_http_url(required(&intent.target, "HTTP URL")?)?;
            Plan::FetchUrl(url)
        }
        Action::RunTool => {
            let program = required(&intent.target, "program")?;
            if !["git", "brew", "cargo"].contains(&program) {
                return Err("That program is not on the allowlist.".into());
            }
            let args = intent.argv.clone().unwrap_or_default();
            Plan::Tool {
                program: program.to_string(),
                args,
            }
        }
        _ => {
            return Err(format!(
                "I understood {:?}, but its trusted executor is not implemented. No changes made.",
                intent.action
            ));
        }
    };
    Ok((risk, plan))
}

fn describe(plan: &Plan) -> String {
    match plan {
        Plan::Read(s) => s.clone(),
        Plan::Open(path) => format!("Open {}", path.display()),
        Plan::OpenUrl(url) => format!("Open {url}"),
        Plan::Trash(path) => format!("Move {} to Trash", path.display()),
        Plan::RemoveAppCompletely(cleanup) => {
            let known_total = cleanup
                .items
                .iter()
                .filter_map(|item| item.bytes)
                .fold(0u64, u64::saturating_add);
            let unknown = cleanup.items.iter().any(|item| item.bytes.is_none());
            let bundle = cleanup
                .bundle_id
                .as_deref()
                .map(|identifier| format!(" · {identifier}"))
                .unwrap_or_default();
            let entries = cleanup
                .items
                .iter()
                .map(|item| {
                    let size = item
                        .bytes
                        .map(system::human_bytes)
                        .unwrap_or_else(|| "size unavailable".into());
                    format!("- {} · {size} · {}", item.category, item.path.display())
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Completely uninstall {}{bundle}\nMove {} matched item(s) to Trash · {}{}\n{}\nShared Group Containers and system-wide /Library helpers are preserved.",
                cleanup.app_name,
                cleanup.items.len(),
                system::human_bytes(known_total),
                if unknown {
                    " plus items of unknown size"
                } else {
                    ""
                },
                entries
            )
        }
        Plan::Move(from, to) => format!("Move {} → {}", from.display(), to.display()),
        Plan::Quit { label, processes } => {
            let pids = processes
                .iter()
                .map(|(pid, _)| pid.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Stop {label} · send SIGTERM to {} process(es): {pids}",
                processes.len()
            )
        }
        Plan::Terminate { pid, process, port } => {
            let subject = match (port, process) {
                (Some(port), Some(process)) => format!("Port {port} · {process} · PID {pid}\n"),
                _ => String::new(),
            };
            format!("{subject}Send SIGTERM to PID {pid}")
        }
        Plan::Cache(name) => format!("Clear {name} cache"),
        Plan::Bluetooth(on) => format!("Turn Bluetooth {}", if *on { "on" } else { "off" }),
        Plan::AirDrop {
            mode,
            enable_bluetooth,
            enable_wifi,
        } => {
            if mode == "Off" {
                return "Turn AirDrop off".into();
            }
            let mut steps = vec![format!("set receiving to {mode}")];
            if *enable_bluetooth {
                steps.insert(0, "turn Bluetooth on".into());
            }
            if let Some(device) = enable_wifi {
                steps.insert(0, format!("turn Wi-Fi on ({device})"));
            }
            format!("Turn AirDrop on · {}", steps.join(" · "))
        }
        Plan::StageManager(on) => {
            format!("Turn Stage Manager {}", if *on { "on" } else { "off" })
        }
        Plan::FetchUrl(url) => format!("GET {url}"),
        Plan::Copy(from, to) => format!("Copy {} → {}", from.display(), to.display()),
        Plan::CreateDir(path) => format!("Create directory {}", path.display()),
        Plan::ChangeDir(path) => format!("{}", path.display()),
        Plan::Tool { program, args } => {
            if args.is_empty() {
                format!("Run {program}")
            } else {
                format!("Run {program} {}", args.join(" "))
            }
        }
        Plan::UpdateCli {
            label,
            executable,
            before_version,
        } => format!(
            "Update {label} · current version {before_version} · run {} upgrade",
            executable.display()
        ),
    }
}

fn confirm(risk: Risk, yes: bool) -> Result<bool, String> {
    if risk == Risk::ReadOnly || risk == Risk::Low {
        return Ok(true);
    }
    if risk == Risk::Destructive && yes {
        return Ok(true);
    }
    print!("Continue? [y/N] ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if risk == Risk::HighRisk {
        Ok(line.trim() == "DELETE")
    } else {
        Ok(matches!(line.trim(), "y" | "Y" | "yes" | "да"))
    }
}

fn execute(plan: Plan) -> Result<(), String> {
    match plan {
        Plan::Read(text) => println!("{text}"),
        Plan::Bluetooth(on) => {
            let value = if on { "1" } else { "0" };
            let output = system::command("blueutil", &["--power", value]).map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    "Bluetooth control requires blueutil (`brew install blueutil`).".to_string()
                } else {
                    error.to_string()
                }
            })?;
            if !output.status.success() {
                return Err(format!(
                    "Could not change Bluetooth power: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            println!("Bluetooth {}", if on { "on" } else { "off" });
        }
        Plan::AirDrop {
            mode,
            enable_bluetooth,
            enable_wifi,
        } => {
            if let Some(device) = enable_wifi {
                system::set_wifi_power(&device, true)
                    .map_err(|error| format!("Could not turn Wi-Fi on for AirDrop: {error}"))?;
            }
            if enable_bluetooth {
                let output = system::command("blueutil", &["--power", "1"]).map_err(|error| {
                    if error.kind() == io::ErrorKind::NotFound {
                        "AirDrop needs Bluetooth; install blueutil (`brew install blueutil`)."
                            .to_string()
                    } else {
                        error.to_string()
                    }
                })?;
                if !output.status.success() {
                    return Err(format!(
                        "Could not turn Bluetooth on for AirDrop: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
            }
            system::set_airdrop_mode(&mode).map_err(|error| error.to_string())?;
            let actual = system::airdrop_mode().map_err(|error| error.to_string())?;
            if actual != mode {
                return Err(format!(
                    "AirDrop mode did not persist (expected {mode}, got {actual})."
                ));
            }
            println!(
                "AirDrop {}",
                if mode == "Off" {
                    "off".to_string()
                } else {
                    format!("on ({mode})")
                }
            );
        }
        Plan::StageManager(on) => {
            system::set_stage_manager(on).map_err(|error| error.to_string())?;
            if system::stage_manager_enabled().map_err(|error| error.to_string())? != on {
                return Err("Stage Manager setting did not persist.".into());
            }
            println!("Stage Manager {}", if on { "on" } else { "off" });
        }
        Plan::FetchUrl(url) => {
            let output = system::command(
                "/usr/bin/curl",
                &[
                    "--silent",
                    "--show-error",
                    "--location",
                    "--connect-timeout",
                    "10",
                    "--max-time",
                    "30",
                    "--max-filesize",
                    "5242880",
                    "--proto",
                    "=http,https",
                    "--proto-redir",
                    "=http,https",
                    "--url",
                    &url,
                ],
            )
            .map_err(|error| error.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                print!("{stdout}");
                if !stdout.ends_with('\n') {
                    println!();
                }
            }
            if !stderr.trim().is_empty() {
                eprint!("{stderr}");
            }
            if !output.status.success() {
                return Err(format!("curl exited with {}", output.status));
            }
        }
        Plan::Open(path) => {
            let status = Command::new("open")
                .arg(&path)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("open failed: {status}"));
            }
        }
        Plan::Trash(path) => match system::trash(&path).map_err(|e| e.to_string())? {
            system::TrashResult::Path(destination) => {
                println!("Moved to {}", destination.display())
            }
            system::TrashResult::Finder => println!("Moved to Trash via Finder"),
        },
        Plan::RemoveAppCompletely(cleanup) => {
            let mut failures = Vec::new();
            let mut moved = 0usize;
            for item in cleanup.items {
                if item.category == "Application" && !system::app_processes(&item.path).is_empty() {
                    return Err(format!(
                        "{} is running. Quit it and retry; no items were moved.",
                        item.path.display()
                    ));
                }
                if fs::symlink_metadata(&item.path).is_err() {
                    continue;
                }
                match system::trash(&item.path) {
                    Ok(system::TrashResult::Path(destination)) => {
                        println!("Moved {} to {}", item.path.display(), destination.display());
                        moved += 1;
                    }
                    Ok(system::TrashResult::Finder) => {
                        println!("Moved {} to Trash via Finder", item.path.display());
                        moved += 1;
                    }
                    Err(error) => failures.push(format!("{}: {error}", item.path.display())),
                }
            }
            if !failures.is_empty() {
                return Err(format!(
                    "Moved {moved} item(s), but {} failed:\n{}",
                    failures.len(),
                    failures.join("\n")
                ));
            }
            println!("Complete uninstall moved {moved} item(s) to Trash.");
        }
        Plan::Move(from, to) => {
            system::rename_no_replace(&from, &to).map_err(|e| e.to_string())?;
            println!("Moved to {}", to.display());
        }
        Plan::Terminate { pid, process, port } => {
            if !system::safe_pid(pid) {
                return Err("Protected PID; no changes made.".into());
            }
            if let Some(port) = port {
                let current = system::port_processes(port).map_err(|e| e.to_string())?;
                if current.len() != 1
                    || current[0].pid != pid
                    || Some(&current[0].command) != process.as_ref()
                {
                    return Err("Port listener changed since preview; no signal sent.".into());
                }
            } else if let Some(expected) = process.as_ref() {
                let current = system::process_by_pid(pid).map_err(|e| e.to_string())?;
                if current.as_ref() != Some(expected) {
                    return Err("Process changed since preview; no signal sent.".into());
                }
            }
            let status = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("SIGTERM failed: {status}"));
            }
            println!("SIGTERM sent to PID {pid}.");
        }
        Plan::OpenUrl(url) => {
            let status = Command::new("open")
                .arg(url)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("open failed: {status}"));
            }
        }
        Plan::Quit { label, processes } => {
            let mut sent = 0;
            for (pid, comm) in processes {
                if !system::safe_pid(pid) {
                    return Err(format!("Protected PID {pid}; no signal sent."));
                }
                let current = system::process_by_pid(pid).map_err(|e| e.to_string())?;
                if current.as_deref() != Some(comm.as_str()) {
                    return Err("Process changed since preview; no signal sent.".into());
                }
                let status = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status()
                    .map_err(|e| e.to_string())?;
                if !status.success() {
                    return Err(format!("SIGTERM failed for PID {pid}: {status}"));
                }
                sent += 1;
            }
            println!("SIGTERM sent to {sent} process(es) of {label}.");
        }
        Plan::Cache(name) => {
            let (program, args): (&str, Vec<&str>) = match name.as_str() {
                "pip" => ("pip", vec!["cache", "purge"]),
                "npm" => ("npm", vec!["cache", "clean", "--force"]),
                "yarn" => ("yarn", vec!["cache", "clean"]),
                "pnpm" => ("pnpm", vec!["store", "prune"]),
                "brew" | "homebrew" => ("brew", vec!["cleanup"]),
                _ => return Err("Unsupported cache".into()),
            };
            let status = Command::new(program)
                .args(args)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Cache command failed: {status}"));
            }
        }
        Plan::Copy(from, to) => {
            system::copy_no_replace(&from, &to).map_err(|e| e.to_string())?;
            println!("Copied to {}", to.display());
        }
        Plan::CreateDir(path) => {
            system::create_directory(&path).map_err(|e| e.to_string())?;
            println!("Created {}", path.display());
        }
        Plan::ChangeDir(path) => {
            println!("{}", path.display());
        }
        Plan::Tool { program, args } => {
            let argv: Vec<&str> = args.iter().map(String::as_str).collect();
            let output = system::command(&program, &argv).map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.trim().is_empty() {
                print!("{stdout}");
                if !stdout.ends_with('\n') {
                    println!();
                }
            }
            if !stderr.trim().is_empty() {
                eprint!("{stderr}");
            }
            if !output.status.success() {
                return Err(format!("{program} exited with {}", output.status));
            }
        }
        Plan::UpdateCli {
            label,
            executable,
            before_version,
        } => {
            let status = Command::new(&executable)
                .arg("upgrade")
                .status()
                .map_err(|error| format!("Could not update {label}: {error}"))?;
            if !status.success() {
                return Err(format!("{label} updater exited with {status}"));
            }
            let output = Command::new(&executable)
                .arg("--version")
                .output()
                .map_err(|error| format!("Could not verify {label}: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "{label} updater completed, but version verification failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            let after_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if after_version == before_version {
                println!("{label} is already up to date ({after_version}).");
            } else {
                println!("{label} updated: {before_version} -> {after_version}.");
            }
        }
    }
    Ok(())
}

fn run_request(
    input: &str,
    dry: bool,
    yes: bool,
    fast_only: bool,
    interpret_only: bool,
) -> Result<(), String> {
    if input.len() > fast::MAX_INPUT_BYTES {
        return Err(format!(
            "Request is longer than {} bytes. No changes made.",
            fast::MAX_INPUT_BYTES
        ));
    }
    let config = config::Config::load()?;
    if fast::has_multiple_steps(input) {
        if let Some(steps) = fast::parse_multi_step(input) {
            return run_multi_step(steps, dry, yes, interpret_only);
        }
        if interpret_only {
            println!(
                "{}",
                serde_json::json!({
                    "steps": [],
                    "executable": false,
                    "reason": "multi-step request recognized, but no safe deterministic plan is available"
                })
            );
            return Ok(());
        }
        return Err("Understood a multi-step request, but safe multi-step planning is not implemented. No changes made.".into());
    }
    let outcome = config
        .fast_parser
        .then(|| fast::parse_outcome(input))
        .flatten();
    let classified = match outcome {
        Some(fast::Parsed::Intent(intent)) => laya::Classified::Executable(intent),
        Some(fast::Parsed::Refused(reason)) if interpret_only => {
            println!(
                "{}",
                serde_json::json!({"action": "UNSUPPORTED", "reason": reason})
            );
            return Ok(());
        }
        Some(fast::Parsed::Refused(reason)) => return Err(reason),
        Some(fast::Parsed::Unsupported(goal)) if interpret_only => {
            println!(
                "{}",
                serde_json::to_string(&goal).map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        Some(fast::Parsed::Unsupported(goal)) => return Err(goal.message()),
        None if fast_only => return Err("No fast-parser match.".into()),
        None if !config.autostart => {
            crate::alias::record_miss(input);
            let russian = input
                .chars()
                .any(|character| ('\u{0400}'..='\u{04ff}').contains(&character));
            return Err(if russian {
                format!(
                    "Пока не понял: {input}\nНичего не выполнено. Формулировка сохранена, чтобы улучшить распознавание."
                )
            } else {
                format!(
                    "Not understood yet: {input}\nNothing was executed. The wording was saved to improve recognition."
                )
            });
        }
        None if dry => {
            let status = laya::status().map_err(|_| {
                "Model-backed dry run needs an already warm daemon; no changes made.".to_string()
            })?;
            if status.get("loaded").and_then(|value| value.as_bool()) != Some(true) {
                return Err(
                    "Model-backed dry run needs an already loaded model; no changes made.".into(),
                );
            }
            laya::classify(input, false, &config)?
        }
        None => laya::classify(input, true, &config)?,
    };
    match classified {
        laya::Classified::Executable(intent) => run_intent(intent, dry, yes, interpret_only),
        laya::Classified::Unsupported(goal) if interpret_only => {
            println!(
                "{}",
                serde_json::to_string(&goal).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        laya::Classified::Unsupported(goal) => Err(goal.message()),
    }
}

fn run_multi_step(
    steps: Vec<Intent>,
    dry: bool,
    yes: bool,
    interpret_only: bool,
) -> Result<(), String> {
    if interpret_only {
        println!("{}", serde_json::json!({"steps": steps}));
        return Ok(());
    }
    let planned: Vec<_> = steps
        .iter()
        .map(|intent| plan(intent, dry))
        .collect::<Result<_, _>>()?;
    if dry {
        println!("Multi-step plan:");
        for (index, (intent, (risk, action))) in steps.iter().zip(&planned).enumerate() {
            println!(
                "{}. {:?} · {:?} · {}",
                index + 1,
                intent.action,
                risk,
                describe(action)
            );
        }
        println!("No changes made.");
        return Ok(());
    }
    for (index, (risk, action)) in planned.into_iter().enumerate() {
        println!("Step {}: {}", index + 1, describe(&action));
        if !confirm(risk, yes)? {
            println!("Cancelled. No further steps run.");
            return Ok(());
        }
        execute(action)?;
    }
    Ok(())
}

fn run_intent(intent: Intent, dry: bool, yes: bool, interpret_only: bool) -> Result<(), String> {
    if interpret_only {
        println!(
            "{}",
            serde_json::to_string(&intent).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let (risk, action) = plan(&intent, dry)?;
    if dry {
        println!(
            "Intent: {:?}\nRisk: {:?}\nPlanned action: {}\nNo changes made.",
            intent.action,
            risk,
            describe(&action)
        );
        return Ok(());
    }
    if !matches!(
        action,
        Plan::Read(_) | Plan::ChangeDir(_) | Plan::Tool { .. }
    ) {
        println!("{}", describe(&action));
    }
    if !confirm(risk, yes)? {
        println!("Cancelled. No changes made.");
        return Ok(());
    }
    execute(action)
}

#[derive(Default)]
struct SessionContext {
    port: Option<u16>,
    apps: Vec<PathBuf>,
    selected_app: Option<PathBuf>,
}

fn reference_intent(input: &str, context: &mut SessionContext) -> Result<Option<Intent>, String> {
    let text = input.trim().to_lowercase();
    if ["первый", "the first one", "first"].contains(&text.as_str()) {
        let app = context
            .apps
            .first()
            .cloned()
            .ok_or("No application choices in this session.")?;
        println!("{} selected.", app.display());
        context.selected_app = Some(app);
        return Ok(None);
    }
    if ["второй", "the second one", "second"].contains(&text.as_str()) {
        let app = context
            .apps
            .get(1)
            .cloned()
            .ok_or("No second application choice.")?;
        println!("{} selected.", app.display());
        context.selected_app = Some(app);
        return Ok(None);
    }
    if ["убей его", "убей это", "kill it", "stop it", "выключи его"].contains(&text.as_str())
    {
        let port = context
            .port
            .ok_or("No process reference in this session.")?;
        let mut intent = Intent::new(Action::KillPortProcess);
        intent.port = Some(port);
        return Ok(Some(intent));
    }
    if [
        "удали его",
        "удали это",
        "remove it",
        "delete it",
        "убери его",
    ]
    .contains(&text.as_str())
    {
        let app = context
            .selected_app
            .as_ref()
            .ok_or("No selected application in this session.")?;
        let name = app
            .file_stem()
            .ok_or("Invalid application reference")?
            .to_string_lossy()
            .to_string();
        return Ok(Some(Intent::target(Action::RemoveApp, name)));
    }
    Ok(None)
}

fn interactive() -> Result<(), String> {
    println!("TerFinder · опиши, что нужно сделать · exit — выйти");
    let mut context = SessionContext::default();
    let config = config::Config::load()?;
    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        if ["exit", "quit", "q"].contains(&line.trim()) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > fast::MAX_INPUT_BYTES {
            eprintln!(
                "Request is longer than {} bytes. No changes made.",
                fast::MAX_INPUT_BYTES
            );
            continue;
        }
        let reference = reference_intent(&line, &mut context);
        let intent = match reference {
            Ok(Some(intent)) => intent,
            Ok(None)
                if [
                    "первый",
                    "второй",
                    "the first one",
                    "the second one",
                    "first",
                    "second",
                ]
                .contains(&line.trim().to_lowercase().as_str()) =>
            {
                continue;
            }
            Ok(None) => match config
                .fast_parser
                .then(|| fast::parse_outcome(&line))
                .flatten()
            {
                Some(fast::Parsed::Intent(intent)) => intent,
                Some(fast::Parsed::Refused(reason)) => {
                    eprintln!("{reason}");
                    continue;
                }
                Some(fast::Parsed::Unsupported(goal)) => {
                    eprintln!("{}", goal.message());
                    continue;
                }
                None if !config.autostart => {
                    crate::alias::record_miss(line.trim());
                    eprintln!("Пока не понял. Ничего не выполнено; формулировка сохранена.");
                    continue;
                }
                None => match laya::classify(&line, true, &config) {
                    Ok(laya::Classified::Executable(intent)) => intent,
                    Ok(laya::Classified::Unsupported(goal)) => {
                        eprintln!("{}", goal.message());
                        continue;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        continue;
                    }
                },
            },
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        match run_intent(intent.clone(), false, false, false) {
            Ok(()) => match intent.action {
                Action::FindPortProcess => context.port = intent.port,
                Action::FindApp | Action::FindProcess => {
                    if let Some(target) = intent.target.as_deref()
                        && let Ok(apps) = system::find_apps(target)
                    {
                        context.selected_app = if apps.len() == 1 {
                            apps.first().cloned()
                        } else {
                            None
                        };
                        context.apps = apps;
                    }
                }
                _ => {}
            },
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}

fn risk_name(risk: Risk) -> &'static str {
    match risk {
        Risk::ReadOnly => "read-only",
        Risk::Low => "low",
        Risk::Destructive => "confirmation",
        Risk::HighRisk => "type DELETE",
    }
}

fn print_capabilities(json: bool) -> Result<(), String> {
    if json {
        let values = intent::CAPABILITIES
            .iter()
            .map(|capability| {
                serde_json::json!({
                    "action": capability.name,
                    "description": capability.description,
                    "required": capability.required,
                    "risk": risk_name(capability.risk),
                    "examples": capability.examples,
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&values).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    println!(
        "TerFinder {} · {} typed actions\n",
        env!("CARGO_PKG_VERSION"),
        intent::CAPABILITIES.len()
    );
    for risk in [Risk::ReadOnly, Risk::Low, Risk::Destructive, Risk::HighRisk] {
        let group = intent::CAPABILITIES
            .iter()
            .filter(|capability| capability.risk == risk)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        println!("{}", risk_name(risk));
        for capability in group {
            println!(
                "  {:<24} {}\n    e.g. tf {}",
                capability.name, capability.description, capability.examples[0]
            );
        }
        println!();
    }
    println!("Preview anything with: tf plan <request>");
    Ok(())
}

fn doctor() -> Result<(), String> {
    println!("TerFinder {} diagnostics", env!("CARGO_PKG_VERSION"));
    println!(
        "[{}] platform: {} {}",
        if cfg!(target_os = "macos") {
            "ok"
        } else {
            "!!"
        },
        env::consts::OS,
        env::consts::ARCH
    );
    let current = env::current_exe().map_err(|error| error.to_string())?;
    println!("[ok] binary: {}", current.display());

    let mut failures = Vec::new();
    let loaded_config = config::Config::load();
    match &loaded_config {
        Ok(config) => println!(
            "[ok] config: fast_parser={}, model_autostart={}, threshold={:.2}",
            config.fast_parser, config.autostart, config.confidence_threshold
        ),
        Err(error) => {
            println!("[!!] config: {error}");
            failures.push("config");
        }
    }
    match crate::knowledge::health_summary() {
        Ok(summary) => println!("[ok] knowledge: {summary}"),
        Err(error) => {
            println!("[!!] knowledge: {error}");
            failures.push("knowledge");
        }
    }

    let required = ["open", "mdfind", "lsof", "ps", "df", "sw_vers", "sysctl"];
    let missing = required
        .into_iter()
        .filter(|name| system::executable(name).is_none())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        println!("[ok] macOS tools: {}", required.join(", "));
    } else {
        println!("[!!] missing required tools: {}", missing.join(", "));
        failures.push("tools");
    }

    let python = loaded_config.as_ref().ok().and_then(laya::python_path);
    match python {
        Some(path) => {
            let ready = Command::new(&path)
                .args(["-c", "import laya_mlx"])
                .output()
                .is_ok_and(|output| output.status.success());
            if ready {
                println!(
                    "[ok] semantic fallback: {} (laya-mlx ready)",
                    path.display()
                );
            } else {
                println!(
                    "[--] semantic fallback: {} (laya-mlx is not installed; native parser still works)",
                    path.display()
                );
            }
        }
        None => println!("[--] semantic fallback: Python not found (native parser still works)"),
    }
    match system::executable("blueutil") {
        Some(path) => println!("[ok] optional Bluetooth helper: {}", path.display()),
        None => println!("[--] optional Bluetooth helper: not installed"),
    }

    let home = system::home().map_err(|error| error.to_string())?;
    let shell_hook = [".zshrc", ".bashrc"]
        .into_iter()
        .map(|name| home.join(name))
        .find(|path| {
            fs::read_to_string(path).is_ok_and(|content| content.contains("# >>> terfinder >>>"))
        });
    match shell_hook {
        Some(path) => println!("[ok] shell integration: {}", path.display()),
        None => println!("[--] shell integration: not installed; run `tf setup` for `tf cd`"),
    }
    match laya::status() {
        Ok(status) => {
            let model = status
                .get("model")
                .and_then(|value| value.as_str())
                .unwrap_or("loaded");
            let pid = status
                .get("pid")
                .and_then(|value| value.as_u64())
                .map(|value| format!(", PID {value}"))
                .unwrap_or_default();
            println!("[ok] semantic model: {model}{pid}");
        }
        Err(_) => println!("[--] semantic model: stopped; it starts on an unfamiliar request"),
    }

    println!("Paths:");
    println!("  config     {}", config::Config::path()?.display());
    println!("  knowledge  {}", crate::knowledge::path()?.display());
    println!(
        "  aliases    {}",
        home.join(".config/terfinder/aliases.toml").display()
    );
    println!(
        "  misses     {}",
        home.join(".cache/terfinder/misses.log").display()
    );
    if failures.is_empty() {
        println!("Result: ready");
        Ok(())
    } else {
        Err(format!("diagnostics failed: {}", failures.join(", ")))
    }
}

fn model_status() -> Result<(), String> {
    match laya::status() {
        Ok(status) => {
            print_model_status(&status);
            Ok(())
        }
        Err(error) if error == "daemon is not running" => {
            println!("Semantic model: stopped");
            println!("It starts automatically when the native parser needs help.");
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn model_stop() -> Result<(), String> {
    match laya::stop() {
        Ok(_) => {
            println!("Semantic model: stopping");
            Ok(())
        }
        Err(error) if error == "daemon is not running" => {
            println!("Semantic model is already stopped.");
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn print_model_status(status: &serde_json::Value) {
    println!("Semantic model: running");
    if let Some(model) = status.get("model").and_then(|value| value.as_str()) {
        println!("  model        {model}");
    }
    if let Some(pid) = status.get("pid").and_then(|value| value.as_u64()) {
        println!("  pid          {pid}");
    }
    if let Some(seconds) = status.get("idle_timeout").and_then(|value| value.as_u64()) {
        println!("  idle timeout {seconds}s");
    }
    if let Some(bytes) = status
        .get("mlx_active_bytes")
        .and_then(|value| value.as_u64())
    {
        println!("  MLX memory   {:.1} MiB", bytes as f64 / 1_048_576.0);
    }
}

fn main() {
    let mut dry = false;
    let mut yes = false;
    let mut fast_only = false;
    let mut interpret_only = false;
    let mut parts = Vec::new();
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                help();
                return;
            }
            "--version" | "-V" => {
                println!("TerFinder {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--resolve-dir" => {
                let rest: Vec<String> = env::args()
                    .skip_while(|arg| arg != "--resolve-dir")
                    .skip(1)
                    .collect();
                if let Err(err) = resolve_dir(&rest.join(" ")) {
                    eprintln!("terfinder: {err}");
                    std::process::exit(1);
                }
                return;
            }
            "--dry-run" => dry = true,
            "--yes" | "-y" => yes = true,
            "--fast-only" => fast_only = true,
            "--interpret-only" => interpret_only = true,
            _ => parts.push(arg),
        }
    }
    let result = match parts.as_slice() {
        [] => interactive(),
        [one] if one == "help" => {
            help();
            Ok(())
        }
        [one] if one == "help-advanced" || one == "advanced" => {
            advanced_help();
            Ok(())
        }
        [one] if one == "version" => {
            println!("TerFinder {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [one] if one == "setup" => setup(),
        [one] if one == "misses" => crate::alias::print_misses(),
        [one] if one == "knowledge" => crate::knowledge::print_status(),
        [one] if one == "capabilities" || one == "examples" => print_capabilities(false),
        [one, flag] if (one == "capabilities" || one == "examples") && flag == "--json" => {
            print_capabilities(true)
        }
        [one, rest @ ..] if one == "plan" && !rest.is_empty() => {
            run_request(&rest.join(" "), true, yes, fast_only, false)
        }
        [one] if one == "plan" => Err("usage: tf plan <request>".into()),
        [one, ..] if one == "remember" => {
            crate::alias::remember_line(&parts[1..].join(" ")).map(|msg| println!("{msg}"))
        }
        [one] if one == "doctor" => doctor(),
        [one] if one == "daemon" || one == "model" => model_status(),
        [a, b] if (a == "daemon" || a == "model") && b == "start" => config::Config::load()
            .and_then(|config| laya::start(&config))
            .map(|status| print_model_status(&status)),
        [a, b] if (a == "daemon" || a == "model") && b == "stop" => model_stop(),
        [a, b] if (a == "daemon" || a == "model") && b == "status" => model_status(),
        [one, ..] if one == "daemon" || one == "model" => {
            Err("usage: tf model <start|stop|status>".into())
        }
        _ => run_request(&parts.join(" "), dry, yes, fast_only, interpret_only),
    };
    if let Err(err) = result {
        eprintln!("terfinder: {err}");
        std::process::exit(1);
    }
}

fn resolve_dir(input: &str) -> Result<(), String> {
    let skip = [
        "cd",
        "chdir",
        "to",
        "in",
        "into",
        "в",
        "на",
        "перейди",
        "перейти",
        "зайди",
        "go",
        "goto",
        "enter",
        "entra",
        "entrar",
        "gehe",
        "the",
        "папку",
        "folder",
        "dir",
    ];
    let stripped = input
        .split_whitespace()
        .filter(|word| !skip.contains(&word.to_lowercase().as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let target = if stripped.is_empty() {
        "~"
    } else {
        stripped.as_str()
    };
    let path = system::existing_dir(target)
        .or_else(|| {
            system::normalize(&system::expand(target).ok()?)
                .ok()
                .filter(|path| path.is_dir())
        })
        .ok_or_else(|| format!("No such directory: {target}"))?;
    println!("{}", path.display());
    Ok(())
}

fn setup() -> Result<(), String> {
    let home = system::home().map_err(|e| e.to_string())?;
    let bin = home.join(".local/bin");
    fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let current = env::current_exe().map_err(|e| e.to_string())?;
    fs::copy(&current, bin.join("tf")).map_err(|e| e.to_string())?;
    fs::copy(&current, bin.join("terfinder")).map_err(|e| e.to_string())?;
    let snippet = include_str!("../scripts/tf-shell.sh");
    let mut hooked = 0usize;
    for rc_name in [".zshrc", ".bashrc"] {
        let rc = home.join(rc_name);
        if !rc.exists() {
            continue;
        }
        let content = fs::read_to_string(&rc).map_err(|e| e.to_string())?;
        if content.contains("# >>> terfinder >>>") {
            println!("Shell hook already present in {}", rc.display());
            hooked += 1;
            continue;
        }
        let mut next = content;
        if !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str("\n# >>> terfinder >>>\n");
        next.push_str(snippet);
        if !snippet.ends_with('\n') {
            next.push('\n');
        }
        next.push_str("# <<< terfinder <<<\n");
        fs::write(&rc, next).map_err(|e| e.to_string())?;
        println!("Added tf shell function to {}", rc.display());
        hooked += 1;
    }
    println!(
        "Installed {} and {}",
        bin.join("tf").display(),
        bin.join("terfinder").display()
    );
    if hooked == 0 {
        println!("No ~/.zshrc or ~/.bashrc found; source scripts/tf-shell.sh yourself.");
    } else {
        println!("Open a new terminal, or `source ~/.zshrc`, so `tf cd` changes this shell.");
    }
    println!("Run `tf doctor` to verify the installation.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_url_validation_allows_only_bounded_http_urls() {
        assert_eq!(
            checked_http_url("https://example.com/search?a=1&b=2").unwrap(),
            "https://example.com/search?a=1&b=2"
        );
        assert!(checked_http_url("file:///etc/passwd").is_err());
        assert!(checked_http_url("https://user:secret@example.com").is_err());
        assert!(checked_http_url("https://example.com/a b").is_err());
    }

    #[test]
    fn dry_run_does_not_move_file() {
        let dir = env::temp_dir().join(format!("terfinder-dry-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.txt");
        fs::write(&path, b"keep me").unwrap();
        let request = format!("удали {}", path.display());
        assert!(run_request(&request, true, false, true, false).is_ok());
        assert_eq!(fs::read(&path).unwrap(), b"keep me");
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn malicious_text_is_not_a_shell_command() {
        for input in [
            "удали /tmp/foo; touch /tmp/pwned",
            "удали $(touch /tmp/pwned)",
            "удали ../../../../System",
        ] {
            if let Some(intent) = fast::parse(input) {
                assert!(plan(&intent, true).is_err());
            }
        }
    }

    #[test]
    fn process_plan_sends_term_first() {
        assert_eq!(
            describe(&Plan::Terminate {
                pid: 12345,
                process: None,
                port: None
            }),
            "Send SIGTERM to PID 12345"
        );
    }

    #[test]
    fn arbitrary_apps_cannot_reach_an_updater() {
        let intent = Intent::target(Action::UpdateApp, "Chrome");
        assert!(plan(&intent, true).is_err());
    }

    #[test]
    fn interactive_references_stay_in_session() {
        let mut context = SessionContext {
            port: Some(8765),
            apps: vec![
                PathBuf::from("/Applications/Google Chrome.app"),
                PathBuf::from("/Applications/Google Chrome Canary.app"),
            ],
            selected_app: None,
        };
        let kill = reference_intent("убей его", &mut context).unwrap().unwrap();
        assert_eq!(kill.action, Action::KillPortProcess);
        assert_eq!(kill.port, Some(8765));
        assert!(reference_intent("первый", &mut context).unwrap().is_none());
        let remove = reference_intent("удали его", &mut context)
            .unwrap()
            .unwrap();
        assert_eq!(remove.action, Action::RemoveApp);
        assert_eq!(remove.target.as_deref(), Some("Google Chrome"));
        assert!(reference_intent("убей его", &mut SessionContext::default()).is_err());
    }
}
