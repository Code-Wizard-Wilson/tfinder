use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    FindApp,
    RemoveApp,
    OpenApp,
    QuitApp,
    FindFile,
    FindFiles,
    OpenFile,
    MoveFile,
    RenameFile,
    DeleteFile,
    DeleteDirectory,
    DirectorySize,
    FindLargeFiles,
    FindProcess,
    KillProcess,
    FindPortProcess,
    KillPortProcess,
    ClearCache,
    ShowDiskUsage,
    ShowBattery,
    ListProcesses,
    SetBluetoothPower,
    #[serde(rename = "SET_AIRDROP_MODE")]
    SetAirDropMode,
    SetStageManager,
    ListDirectory,
    ReadFile,
    CopyFile,
    CreateDirectory,
    PrintWorkingDirectory,
    ChangeDirectory,
    ShowDate,
    WhoAmI,
    ShowHostname,
    ShowSystemInfo,
    ShowUptime,
    ShowMemory,
    ShowCpu,
    ShowNetwork,
    FetchUrl,
    RunTool,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileType {
    Video,
    Pdf,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnsupportedGoal {
    SetDefaultBrowser,
    EnableLoginItem,
    DisableLoginItem,
    DownloadFile,
    InstallApp,
    UpdateOs,
    SendEmail,
    CreateAccount,
}

impl UnsupportedGoal {
    pub fn name(self) -> &'static str {
        match self {
            Self::SetDefaultBrowser => "SET_DEFAULT_BROWSER",
            Self::EnableLoginItem => "ENABLE_LOGIN_ITEM",
            Self::DisableLoginItem => "DISABLE_LOGIN_ITEM",
            Self::DownloadFile => "DOWNLOAD_FILE",
            Self::InstallApp => "INSTALL_APP",
            Self::UpdateOs => "UPDATE_OS",
            Self::SendEmail => "SEND_EMAIL",
            Self::CreateAccount => "CREATE_ACCOUNT",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SetDefaultBrowser => "change the default browser",
            Self::EnableLoginItem => "enable an app at login",
            Self::DisableLoginItem => "disable an app at login",
            Self::DownloadFile => "download a file",
            Self::InstallApp => "install an application",
            Self::UpdateOs => "update macOS",
            Self::SendEmail => "send an email",
            Self::CreateAccount => "create an account",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SemanticGoal {
    pub action: &'static str,
    pub semantic_goal: UnsupportedGoal,
    pub target: Option<String>,
    pub confidence: f32,
}

impl SemanticGoal {
    pub fn message(&self) -> String {
        let object = self
            .target
            .as_deref()
            .map(|target| format!(" for {target}"))
            .unwrap_or_default();
        format!(
            "Understood: {}{object}. The {} executor is not implemented. No changes made.",
            self.semantic_goal.description(),
            self.semantic_goal.name()
        )
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    ReadOnly,
    Low,
    Destructive,
    HighRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub action: Action,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub minimum_size_bytes: Option<u64>,
    #[serde(default)]
    pub file_type: Option<FileType>,
    #[serde(default)]
    pub file_extension: Option<String>,
    #[serde(default)]
    pub name_contains: Option<String>,
    #[serde(default)]
    pub max_age_days: Option<u32>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub permanent: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub long: bool,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub argv: Option<Vec<String>>,
}

impl Intent {
    pub fn new(action: Action) -> Self {
        Self {
            action,
            target: None,
            source: None,
            destination: None,
            port: None,
            pid: None,
            minimum_size_bytes: None,
            file_type: None,
            file_extension: None,
            name_contains: None,
            max_age_days: None,
            limit: None,
            permanent: false,
            hidden: false,
            long: false,
            language: None,
            confidence: None,
            argv: None,
        }
    }
    pub fn target(action: Action, target: impl Into<String>) -> Self {
        let mut intent = Self::new(action);
        intent.target = Some(target.into());
        intent
    }
}

pub struct Capability {
    pub action: Action,
    pub name: &'static str,
    pub description: &'static str,
    pub required: &'static str,
    pub risk: Risk,
    pub examples: &'static [&'static str],
}

pub const CAPABILITIES: &[Capability] = &[
    Capability {
        action: Action::FindApp,
        name: "FIND_APP",
        description: "Locate an installed macOS app",
        required: "app name",
        risk: Risk::ReadOnly,
        examples: &["найди приложение Chrome", "find app Firefox"],
    },
    Capability {
        action: Action::RemoveApp,
        name: "REMOVE_APP",
        description: "Move an installed app to Trash",
        required: "app name",
        risk: Risk::Destructive,
        examples: &["удали Chrome", "uninstall Firefox"],
    },
    Capability {
        action: Action::OpenApp,
        name: "OPEN_APP",
        description: "Open an installed app",
        required: "app name",
        risk: Risk::Low,
        examples: &["открой Chrome", "запусти Safari"],
    },
    Capability {
        action: Action::QuitApp,
        name: "QUIT_APP",
        description: "Quit a running app or process by name",
        required: "app name",
        risk: Risk::Destructive,
        examples: &["закрой Telegram", "останови Discord"],
    },
    Capability {
        action: Action::FindFile,
        name: "FIND_FILE",
        description: "Find a file in bounded locations",
        required: "file name",
        risk: Risk::ReadOnly,
        examples: &["найди report.pdf", "where is package.json"],
    },
    Capability {
        action: Action::FindFiles,
        name: "FIND_FILES",
        description: "Find files recursively within one bounded directory",
        required: "directory and file name or extension",
        risk: Risk::ReadOnly,
        examples: &[
            "найди здесь все файлы с расширением json",
            "find README in the current project",
        ],
    },
    Capability {
        action: Action::OpenFile,
        name: "OPEN_FILE",
        description: "Open a known file",
        required: "file path",
        risk: Risk::Low,
        examples: &["открой hosts", "open report.pdf"],
    },
    Capability {
        action: Action::MoveFile,
        name: "MOVE_FILE",
        description: "Move a file without overwriting",
        required: "source, destination",
        risk: Risk::Destructive,
        examples: &["перемести test.txt на Desktop", "move test.txt to Desktop"],
    },
    Capability {
        action: Action::RenameFile,
        name: "RENAME_FILE",
        description: "Rename a file without overwriting",
        required: "source, destination",
        risk: Risk::Destructive,
        examples: &["переименуй a.txt в b.txt", "rename a.txt to b.txt"],
    },
    Capability {
        action: Action::DeleteFile,
        name: "DELETE_FILE",
        description: "Move a file to Trash",
        required: "file path",
        risk: Risk::Destructive,
        examples: &["удали test.txt", "delete test.txt"],
    },
    Capability {
        action: Action::DeleteDirectory,
        name: "DELETE_DIRECTORY",
        description: "Move a directory to Trash",
        required: "directory path",
        risk: Risk::Destructive,
        examples: &["удали папку node_modules", "rm -r build"],
    },
    Capability {
        action: Action::DirectorySize,
        name: "DIRECTORY_SIZE",
        description: "Show directory size",
        required: "directory",
        risk: Risk::ReadOnly,
        examples: &["сколько весит Downloads", "size Downloads"],
    },
    Capability {
        action: Action::FindLargeFiles,
        name: "FIND_LARGE_FILES",
        description: "Find largest files within a directory",
        required: "directory",
        risk: Risk::ReadOnly,
        examples: &["show largest files in Downloads"],
    },
    Capability {
        action: Action::FindProcess,
        name: "FIND_PROCESS",
        description: "Locate one specific process by its name",
        required: "process name",
        risk: Risk::ReadOnly,
        examples: &["найди python", "find process node"],
    },
    Capability {
        action: Action::KillProcess,
        name: "KILL_PROCESS",
        description: "Send SIGTERM to a selected process",
        required: "PID",
        risk: Risk::Destructive,
        examples: &["убей PID 123", "kill process 123"],
    },
    Capability {
        action: Action::FindPortProcess,
        name: "FIND_PORT_PROCESS",
        description: "Show TCP listener on a port",
        required: "port",
        risk: Risk::ReadOnly,
        examples: &["кто на порту 8765", "who uses port 8765"],
    },
    Capability {
        action: Action::KillPortProcess,
        name: "KILL_PORT_PROCESS",
        description: "Send SIGTERM to TCP listener",
        required: "port",
        risk: Risk::Destructive,
        examples: &["освободи порт 8765", "free port 8765"],
    },
    Capability {
        action: Action::ClearCache,
        name: "CLEAR_CACHE",
        description: "Clear a supported named cache",
        required: "cache name",
        risk: Risk::Destructive,
        examples: &["очисти кеш pip", "clear npm cache"],
    },
    Capability {
        action: Action::ShowDiskUsage,
        name: "SHOW_DISK_USAGE",
        description: "Show disk usage",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["покажи место на диске", "disk usage"],
    },
    Capability {
        action: Action::ShowBattery,
        name: "SHOW_BATTERY",
        description: "Show battery charge and power source",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["сколько у меня зарядки", "battery level"],
    },
    Capability {
        action: Action::ListProcesses,
        name: "LIST_PROCESSES",
        description: "List many or all running processes, especially ranked by CPU or memory usage",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &[
            "найди работающие процессы",
            "show running processes",
            "какой процесс занимает больше всего памяти",
            "top CPU processes",
        ],
    },
    Capability {
        action: Action::SetBluetoothPower,
        name: "SET_BLUETOOTH_POWER",
        description: "Turn Bluetooth on or off using blueutil",
        required: "on or off",
        risk: Risk::Low,
        examples: &["включи блютуз", "выключи bluetooth"],
    },
    Capability {
        action: Action::SetAirDropMode,
        name: "SET_AIRDROP_MODE",
        description: "Turn AirDrop receiving off or allow contacts/everyone",
        required: "off, contacts, everyone, or on",
        risk: Risk::Destructive,
        examples: &["включи AirDrop", "AirDrop только для контактов"],
    },
    Capability {
        action: Action::SetStageManager,
        name: "SET_STAGE_MANAGER",
        description: "Turn macOS Stage Manager on or off",
        required: "on or off",
        risk: Risk::Low,
        examples: &["включи Stage Manager", "turn off Stage Manager"],
    },
    Capability {
        action: Action::ListDirectory,
        name: "LIST_DIRECTORY",
        description: "List files in a directory",
        required: "directory or current directory",
        risk: Risk::ReadOnly,
        examples: &[
            "ls",
            "покажи файлы",
            "list files in Downloads",
            "lista archivos",
        ],
    },
    Capability {
        action: Action::ReadFile,
        name: "READ_FILE",
        description: "Print a text file",
        required: "file path",
        risk: Risk::ReadOnly,
        examples: &["cat README.md", "прочитай hosts", "read package.json"],
    },
    Capability {
        action: Action::CopyFile,
        name: "COPY_FILE",
        description: "Copy a file without overwriting",
        required: "source, destination",
        risk: Risk::Low,
        examples: &["cp a.txt b.txt", "скопируй test.txt на Desktop"],
    },
    Capability {
        action: Action::CreateDirectory,
        name: "CREATE_DIRECTORY",
        description: "Create a directory",
        required: "directory path",
        risk: Risk::Low,
        examples: &["mkdir tmp", "создай папку notes"],
    },
    Capability {
        action: Action::PrintWorkingDirectory,
        name: "PRINT_WORKING_DIRECTORY",
        description: "Show the current directory",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["pwd", "где я", "where am i"],
    },
    Capability {
        action: Action::ChangeDirectory,
        name: "CHANGE_DIRECTORY",
        description: "Resolve a directory for the shell to enter",
        required: "directory",
        risk: Risk::ReadOnly,
        examples: &["cd Downloads", "перейди в Documents"],
    },
    Capability {
        action: Action::ShowDate,
        name: "SHOW_DATE",
        description: "Show the current date and time",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["date", "какое сегодня число"],
    },
    Capability {
        action: Action::WhoAmI,
        name: "WHO_AM_I",
        description: "Show the current user name",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["whoami", "кто я"],
    },
    Capability {
        action: Action::ShowHostname,
        name: "SHOW_HOSTNAME",
        description: "Show the computer hostname",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["hostname", "как называется компьютер"],
    },
    Capability {
        action: Action::ShowSystemInfo,
        name: "SHOW_SYSTEM_INFO",
        description: "Show macOS version, architecture, host, and kernel",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["system info", "информация о системе", "sw_vers"],
    },
    Capability {
        action: Action::ShowUptime,
        name: "SHOW_UPTIME",
        description: "Show how long the Mac has been running",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["uptime", "сколько работает компьютер"],
    },
    Capability {
        action: Action::ShowMemory,
        name: "SHOW_MEMORY",
        description: "Show physical memory and current VM statistics",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["memory usage", "сколько занято памяти"],
    },
    Capability {
        action: Action::ShowCpu,
        name: "SHOW_CPU",
        description: "Show processor model and core counts",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["cpu info", "какой процессор"],
    },
    Capability {
        action: Action::ShowNetwork,
        name: "SHOW_NETWORK",
        description: "Show active local network interfaces and addresses",
        required: "none",
        risk: Risk::ReadOnly,
        examples: &["network info", "покажи ip адрес"],
    },
    Capability {
        action: Action::FetchUrl,
        name: "FETCH_URL",
        description: "Make a safe HTTP or HTTPS GET request and print the response body",
        required: "http or https URL",
        risk: Risk::Low,
        examples: &[
            "сделай curl запрос на https://example.com",
            "fetch https://example.com",
        ],
    },
    Capability {
        action: Action::RunTool,
        name: "RUN_TOOL",
        description: "Run an allowlisted developer tool with fixed subcommands",
        required: "program and subcommand",
        risk: Risk::ReadOnly,
        examples: &["git status", "brew outdated", "cargo test"],
    },
];

pub fn capability(action: Action) -> Option<&'static Capability> {
    CAPABILITIES.iter().find(|cap| cap.action == action)
}

pub fn validate_model_json(raw: &str, threshold: f32) -> Option<Intent> {
    let intent: Intent = serde_json::from_str(raw).ok()?;
    let confidence = intent.confidence?;
    if !confidence.is_finite() || confidence < threshold || confidence > 1.0 {
        return None;
    }
    capability(intent.action)?;
    Some(intent)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_shell_and_low_confidence() {
        assert!(
            validate_model_json(
                r#"{"action":"REMOVE_APP","target":"Chrome","confidence":0.99,"shell":"rm -rf /"}"#,
                0.7
            )
            .is_none()
        );
        assert!(
            validate_model_json(
                r#"{"action":"REMOVE_APP","target":"Chrome","confidence":0.2}"#,
                0.7
            )
            .is_none()
        );
    }

    #[test]
    fn capability_registry_has_unique_names_and_actions() {
        let mut names = std::collections::HashSet::new();
        for (index, capability) in CAPABILITIES.iter().enumerate() {
            assert!(
                names.insert(capability.name),
                "duplicate {}",
                capability.name
            );
            assert!(
                !capability.examples.is_empty(),
                "{} needs an example",
                capability.name
            );
            assert!(
                !CAPABILITIES[..index]
                    .iter()
                    .any(|previous| previous.action == capability.action),
                "duplicate action {:?}",
                capability.action
            );
        }
    }
}
