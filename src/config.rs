use std::{fs, path::PathBuf};

#[derive(Clone)]
pub struct Config {
    pub confidence_threshold: f32,
    pub idle_timeout_seconds: u64,
    pub model_name: String,
    pub fast_parser: bool,
    pub autostart: bool,
    pub python: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.70,
            idle_timeout_seconds: 600,
            model_name: "multilingual".into(),
            fast_parser: true,
            autostart: true,
            python: None,
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf, String> {
        Ok(crate::system::home()
            .map_err(|e| e.to_string())?
            .join(".config/terfinder/config.toml"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("could not read {}: {error}", path.display())),
        };
        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self, String> {
        let mut config = Self::default();
        let mut section = "";
        for (index, line) in content.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = &line[1..line.len() - 1];
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("config line {}: expected key = value", index + 1))?;
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match (section, key) {
                ("intent", "confidence_threshold") => {
                    config.confidence_threshold =
                        value.parse().map_err(|_| "invalid confidence_threshold")?;
                    if !config.confidence_threshold.is_finite()
                        || !(0.5..=1.0).contains(&config.confidence_threshold)
                    {
                        return Err("confidence_threshold must be 0.5..=1.0".into());
                    }
                }
                ("intent", "fast_parser") => {
                    config.fast_parser = value
                        .parse()
                        .map_err(|_| "fast_parser must be true or false")?;
                }
                ("model", "autostart") => {
                    config.autostart = value
                        .parse()
                        .map_err(|_| "autostart must be true or false")?;
                }
                ("model", "idle_timeout_seconds") => {
                    config.idle_timeout_seconds =
                        value.parse().map_err(|_| "invalid idle_timeout_seconds")?;
                    if !(60..=3600).contains(&config.idle_timeout_seconds) {
                        return Err("idle_timeout_seconds must be 60..=3600".into());
                    }
                }
                ("model", "name") => {
                    if !["multilingual", "english", "typed-decisions"].contains(&value) {
                        return Err(
                            "model.name must be multilingual, english, or typed-decisions".into(),
                        );
                    }
                    config.model_name = value.into();
                }
                ("model", "python") => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() {
                        return Err("model.python must be an absolute path".into());
                    }
                    config.python = Some(path);
                }
                ("model", "transport") if value == "unix" => {}
                ("model", "transport") => {
                    return Err("only Unix-socket transport is supported".into());
                }
                ("", "language") if ["auto", "en", "ru"].contains(&value) => {}
                ("", "color") if ["true", "false"].contains(&value) => {}
                ("", "confirm_destructive") if value == "true" => {}
                ("", "confirm_destructive") => {
                    return Err("confirm_destructive cannot be disabled".into());
                }
                _ => return Err(format!("unsupported config key [{section}] {key}")),
            }
        }
        Ok(config)
    }

    pub fn checkpoint(&self) -> &'static str {
        match self.model_name.as_str() {
            "english" => "aac6fef/laya-mlx",
            "typed-decisions" => "aac6fef/laya-typed-decisions-mlx",
            _ => "aac6fef/laya-multilingual-mlx",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_model_settings() {
        let config = Config::parse("[intent]\nconfidence_threshold = 0.8\nfast_parser = false\n[model]\nname = \"english\"\nidle_timeout_seconds = 120\ntransport = \"unix\"\n").unwrap();
        assert_eq!(config.confidence_threshold, 0.8);
        assert!(!config.fast_parser);
        assert_eq!(config.idle_timeout_seconds, 120);
        assert_eq!(config.checkpoint(), "aac6fef/laya-mlx");
    }
    #[test]
    fn rejects_unsafe_values() {
        assert!(Config::parse("[intent]\nconfidence_threshold = 0.1").is_err());
        assert!(Config::parse("[model]\ntransport = \"http\"").is_err());
        assert!(Config::parse("[model]\npython = \"python3\"").is_err());
        assert_eq!(
            Config::parse("[model]\npython = \"/usr/bin/python3\"")
                .unwrap()
                .python,
            Some(PathBuf::from("/usr/bin/python3"))
        );
    }
}
