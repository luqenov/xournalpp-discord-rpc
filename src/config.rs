use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CLIENT_ID: &str = "1561027274977972367";
pub const DEFAULT_INTERVAL: u64 = 15;
pub const MIN_INTERVAL: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub client_id: String,
    pub interval: u64,
    pub show_document: bool,
    pub verbose: bool,
    pub once: bool,
    pub dry_run: bool,
    pub help: bool,
    pub version: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
            interval: DEFAULT_INTERVAL,
            show_document: true,
            verbose: false,
            once: false,
            dry_run: false,
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
        let mut config = Self::from_env_and_file();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => config.help = true,
                "-V" | "--version" => config.version = true,
                "-v" | "--verbose" => config.verbose = true,
                "--once" => config.once = true,
                "--dry-run" => config.dry_run = true,
                "--no-document" => config.show_document = false,
                "--document" => config.show_document = true,
                "--client-id" => {
                    config.client_id = args.next().ok_or("--client-id requires a value")?;
                }
                "--interval" => {
                    let value = args.next().ok_or("--interval requires a value")?;
                    config.interval = parse_interval(&value)?;
                }
                _ if arg.starts_with("--client-id=") => {
                    config.client_id = arg["--client-id=".len()..].to_string();
                }
                _ if arg.starts_with("--interval=") => {
                    config.interval = parse_interval(&arg["--interval=".len()..])?;
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }
        Ok(config)
    }

    fn from_env_and_file() -> Self {
        let mut config = Self::default();

        if let Some(path) = client_id_file_path() {
            if let Ok(value) = fs::read_to_string(path) {
                let value = value.trim();
                if !value.is_empty() {
                    config.client_id = value.to_string();
                }
            }
        }

        if let Some(path) = config_file_path() {
            for (key, value) in read_key_values(&path) {
                config.apply(&key, &value);
            }
        }

        if let Ok(value) = env::var("XOURNAL_RPC_CLIENT_ID") {
            if !value.trim().is_empty() {
                config.client_id = value.trim().to_string();
            }
        }
        if let Ok(value) = env::var("XOURNAL_RPC_INTERVAL") {
            if let Ok(interval) = parse_interval(&value) {
                config.interval = interval;
            }
        }
        if let Ok(value) = env::var("XOURNAL_RPC_SHOW_DOCUMENT") {
            if let Some(show) = parse_bool(&value) {
                config.show_document = show;
            }
        }
        if let Ok(value) = env::var("XOURNAL_RPC_VERBOSE") {
            if let Some(verbose) = parse_bool(&value) {
                config.verbose = verbose;
            }
        }

        config
    }

    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "client_id" => self.client_id = value.to_string(),
            "interval" => {
                if let Ok(interval) = parse_interval(value) {
                    self.interval = interval;
                }
            }
            "show_document" => {
                if let Some(show) = parse_bool(value) {
                    self.show_document = show;
                }
            }
            "verbose" => {
                if let Some(verbose) = parse_bool(value) {
                    self.verbose = verbose;
                }
            }
            _ => {}
        }
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

fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config"))
}

fn client_id_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("client_id"))
}

fn read_key_values(path: &Path) -> Vec<(String, String)> {
    fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        return None;
                    }
                    let (key, value) = line.split_once('=')?;
                    Some((key.trim().to_string(), value.trim().to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
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
        "      --dry-run           Print detected status without connecting to Discord\n",
        "      --no-document       Do not share the document name\n",
        "      --client-id <ID>    Discord application client ID\n",
        "      --interval <SECS>   Polling interval in seconds [default: 15]\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bools() {
        for value in ["1", "true", "YES", "on"] {
            assert_eq!(parse_bool(value), Some(true));
        }
        for value in ["0", "false", "NO", "off"] {
            assert_eq!(parse_bool(value), Some(false));
        }
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn validates_client_ids() {
        assert!(valid_client_id("1561027274977972367"));
        assert!(!valid_client_id(""));
        assert!(!valid_client_id("abc"));
        assert!(!valid_client_id("1234"));
    }

    #[test]
    fn parses_cli_options() {
        let config = Config::from_args([
            "--no-document".to_string(),
            "--interval=30".to_string(),
            "--client-id".to_string(),
            "1561027274977972367".to_string(),
        ])
        .unwrap();
        assert!(!config.show_document);
        assert_eq!(config.interval, 30);
        assert_eq!(config.client_id, "1561027274977972367");
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(Config::from_args(["--wat".to_string()]).is_err());
    }
}
