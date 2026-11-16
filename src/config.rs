use serde::Deserialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_CLIENT_ID: &str = "1561027274977972367";
pub const DEFAULT_INTERVAL: u64 = 15;
pub const MIN_INTERVAL: u64 = 2;

pub const CONFIG_TEMPLATE: &str = r#"# Message shown in Discord (2-128 characters).
message = "Studying"
# Include the message in the activity; disable to show only other enabled fields.
show_message = true
# Seconds between updates. Changes to this file apply at the next update.
interval = 15
# Share the current document name.
show_document = true
# Show the Codeberg repository button.
show_codeberg = true
# Show elapsed time since Xournal++ started.
show_timer = true
# Hide the activity unless a Xournal++ window has focus.
only_when_focused = false
# Print connection and window detection diagnostics.
verbose = false
# Optional custom Discord application (keep the ID quoted).
# client_id = "1561027274977972367"
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub client_id: String,
    pub message: String,
    pub show_message: bool,
    pub interval: u64,
    pub show_document: bool,
    pub show_codeberg: bool,
    pub show_timer: bool,
    pub only_when_focused: bool,
    pub verbose: bool,
    pub once: bool,
    pub dry_run: bool,
    pub init_config: bool,
    pub help: bool,
    pub version: bool,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    client_id: Option<String>,
    message: Option<String>,
    show_message: Option<bool>,
    interval: Option<u64>,
    show_document: Option<bool>,
    show_codeberg: Option<bool>,
    show_timer: Option<bool>,
    only_when_focused: Option<bool>,
    verbose: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
            message: "Studying".to_string(),
            show_message: true,
            interval: DEFAULT_INTERVAL,
            show_document: true,
            show_codeberg: true,
            show_timer: true,
            only_when_focused: false,
            verbose: false,
            once: false,
            dry_run: false,
            init_config: false,
            help: false,
            version: false,
        }
    }
}

impl Config {
    pub fn from_args<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let args: Vec<String> = args.into_iter().collect();
        let mut config = Self::default();
        config.apply_args(args.clone())?;
        // Help and config creation must work even if an existing config is invalid.
        if config.help || config.version || config.init_config {
            return Ok(config);
        }
        let mut config = Self::from_env_and_file()?;
        config.apply_args(args)?;
        Ok(config)
    }

    fn apply_args(&mut self, args: Vec<String>) -> Result<(), String> {
        let config = self;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => config.help = true,
                "-V" | "--version" => config.version = true,
                "-v" | "--verbose" => config.verbose = true,
                "--once" => config.once = true,
                "--dry-run" => config.dry_run = true,
                "--init-config" => config.init_config = true,
                "--no-message" => config.show_message = false,
                "--show-message" => config.show_message = true,
                "--no-document" => config.show_document = false,
                "--document" => config.show_document = true,
                "--no-codeberg" => config.show_codeberg = false,
                "--codeberg" => config.show_codeberg = true,
                "--no-timer" => config.show_timer = false,
                "--timer" => config.show_timer = true,
                "--only-when-focused" => config.only_when_focused = true,
                "--always" => config.only_when_focused = false,
                "--message" => config.message = args.next().ok_or("--message requires a value")?,
                "--client-id" => {
                    config.client_id = args.next().ok_or("--client-id requires a value")?
                }
                "--interval" => {
                    config.interval =
                        parse_interval(&args.next().ok_or("--interval requires a value")?)?
                }
                _ if arg.starts_with("--message=") => {
                    config.message = arg["--message=".len()..].to_string()
                }
                _ if arg.starts_with("--client-id=") => {
                    config.client_id = arg["--client-id=".len()..].to_string()
                }
                _ if arg.starts_with("--interval=") => {
                    config.interval = parse_interval(&arg["--interval=".len()..])?
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }
        Ok(())
    }

    fn from_env_and_file() -> Result<Self, String> {
        let mut config = Self::default();
        if let Some(path) = config_dir().map(|dir| dir.join("client_id")) {
            if let Ok(value) = fs::read_to_string(path) {
                if !value.trim().is_empty() {
                    config.client_id = value.trim().to_string();
                }
            }
        }
        if let Some(path) = config_file_path() {
            config.load_file(&path)?;
        }
        for (key, variable) in [
            ("client_id", "XOURNAL_RPC_CLIENT_ID"),
            ("message", "XOURNAL_RPC_MESSAGE"),
            ("show_message", "XOURNAL_RPC_SHOW_MESSAGE"),
            ("interval", "XOURNAL_RPC_INTERVAL"),
            ("show_document", "XOURNAL_RPC_SHOW_DOCUMENT"),
            ("show_codeberg", "XOURNAL_RPC_SHOW_CODEBERG"),
            ("show_timer", "XOURNAL_RPC_SHOW_TIMER"),
            ("only_when_focused", "XOURNAL_RPC_ONLY_WHEN_FOCUSED"),
            ("verbose", "XOURNAL_RPC_VERBOSE"),
        ] {
            if let Ok(value) = env::var(variable) {
                config
                    .apply(key, &value)
                    .map_err(|error| format!("{variable}: {error}"))?;
            }
        }
        Ok(config)
    }

    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        self.apply_toml(&content)
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    fn apply_toml(&mut self, content: &str) -> Result<(), String> {
        let file: FileConfig = toml::from_str(content).map_err(|error| error.to_string())?;
        macro_rules! apply {
            ($($field:ident),*) => { $(if let Some(value) = file.$field { self.$field = value; })* };
        }
        apply!(
            client_id,
            message,
            show_message,
            interval,
            show_document,
            show_codeberg,
            show_timer,
            only_when_focused,
            verbose
        );
        Ok(())
    }

    fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        let boolean = || parse_bool(value).ok_or_else(|| format!("invalid boolean: {value:?}"));
        match key {
            "client_id" => self.client_id = value.trim().to_string(),
            "message" => self.message = value.to_string(),
            "interval" => self.interval = parse_interval(value)?,
            "show_message" => self.show_message = boolean()?,
            "show_document" => self.show_document = boolean()?,
            "show_codeberg" => self.show_codeberg = boolean()?,
            "show_timer" => self.show_timer = boolean()?,
            "only_when_focused" => self.only_when_focused = boolean()?,
            "verbose" => self.verbose = boolean()?,
            _ => return Err(format!("unknown setting: {key}")),
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !valid_client_id(&self.client_id) {
            return Err(format!(
                "invalid Discord client ID {:?}: expected 17-20 digits",
                self.client_id
            ));
        }
        if self.interval < MIN_INTERVAL {
            return Err(format!(
                "invalid interval {}: minimum is {} seconds",
                self.interval, MIN_INTERVAL
            ));
        }
        if self.show_message
            && (!(2..=128).contains(&self.message.chars().count())
                || self.message.trim().is_empty())
        {
            return Err("message must contain 2-128 characters and cannot be blank; use --no-message or show_message = false to hide it".to_string());
        }
        Ok(())
    }
}

pub fn valid_client_id(id: &str) -> bool {
    (17..=20).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit())
}

fn parse_interval(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid interval: {value:?}"))
}

pub fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|path| path.join("xournal-rpc"))
}

pub fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

pub fn init_config(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            format!(
                "{}: {error}; existing files are never overwritten",
                path.display()
            )
        })?;
    file.write_all(CONFIG_TEMPLATE.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))
}

pub fn help() -> &'static str {
    concat!(
        "xournal-rpc ",
        env!("CARGO_PKG_VERSION"),
        "\n",
        "Discord Rich Presence for Xournal++ on Linux.\n\n",
        "Usage: xournal-rpc [OPTIONS]\n\n",
        "Options:\n",
        "  -h, --help              Show this help\n",
        "  -V, --version           Show version\n",
        "  -v, --verbose           Print diagnostic messages\n",
        "      --once              Run one update cycle and exit\n",
        "      --dry-run           Preview status and sharing settings without Discord\n",
        "      --init-config       Create a commented config.toml without overwriting\n",
        "      --message <TEXT>    Activity message [default: Studying]\n",
        "      --no-message        Hide the activity message\n",
        "      --show-message      Show the configured activity message [default]\n",
        "      --no-document       Do not share the document name\n",
        "      --document          Share the document name [default]\n",
        "      --no-codeberg       Hide the Codeberg button\n",
        "      --codeberg          Show the Codeberg button [default]\n",
        "      --no-timer          Hide elapsed session time\n",
        "      --timer             Show elapsed session time [default]\n",
        "      --only-when-focused Hide activity while Xournal++ is in the background\n",
        "      --always            Show activity in the background too [default]\n",
        "      --client-id <ID>    Discord application client ID\n",
        "      --interval <SECS>   Polling interval in seconds [default: 15]\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bools_and_validates_ids() {
        for value in ["1", "true", "YES", "on"] {
            assert_eq!(parse_bool(value), Some(true));
        }
        for value in ["0", "false", "NO", "off"] {
            assert_eq!(parse_bool(value), Some(false));
        }
        assert_eq!(parse_bool("maybe"), None);
        assert!(valid_client_id(DEFAULT_CLIENT_ID));
        for value in ["", "abc", "1234"] {
            assert!(!valid_client_id(value));
        }
    }

    #[test]
    fn reads_typed_toml_and_rejects_errors_without_partial_changes() {
        let mut config = Config::default();
        config.apply_toml("message = 'Revisando matemática'\ninterval = 30\nshow_codeberg = false\nshow_timer = false\nonly_when_focused = true").unwrap();
        assert_eq!(config.message, "Revisando matemática");
        assert_eq!(config.interval, 30);
        assert!(!config.show_codeberg);
        assert!(!config.show_timer);
        assert!(config.only_when_focused);
        let before = config.clone();
        for content in [
            "message = [",
            "show_timer = 'false'",
            "interval = -1",
            "show_timre = false",
            "client_id = 1561027274977972367",
        ] {
            assert!(config.apply_toml(content).is_err(), "{content}");
            assert_eq!(config, before);
        }
    }

    #[test]
    fn validates_message_boundaries() {
        let mut config = Config::default();
        for message in ["", "a", "  ", &"x".repeat(129)] {
            config.message = message.to_string();
            assert!(config.validate().is_err());
        }
        for message in ["ab".to_string(), "á".repeat(128)] {
            config.message = message;
            config.validate().unwrap();
        }
    }

    #[test]
    fn template_is_valid_and_never_overwrites_existing_files() {
        let path = env::temp_dir().join(format!("xournal-rpc-init-{}.toml", std::process::id()));
        init_config(&path).unwrap();
        let mut config = Config::default();
        config.load_file(&path).unwrap();
        config.validate().unwrap();
        assert_eq!(config, Config::default());
        fs::write(&path, "keep me").unwrap();
        assert!(init_config(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep me");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_environment_values_return_errors() {
        let mut config = Config::default();
        assert!(config.apply("show_timer", "maybe").is_err());
        assert!(config.apply("interval", "tomorrow").is_err());
        config.apply("show_timer", "off").unwrap();
        assert!(!config.show_timer);
    }
}
