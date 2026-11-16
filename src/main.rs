mod config;
mod discord;
mod ipc;
mod process;
mod window;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use config::Config;
use discord::Presence;
use window::WindowDetector;

#[derive(Debug, Clone)]
pub struct Status {
    pub running: bool,
    pub focused: bool,
    pub document: Option<String>,
    pub started: Option<i64>,
    pub pid: Option<u32>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config = match Config::from_args(args.clone()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("xournal-rpc: {error}");
            std::process::exit(2);
        }
    };

    if config.help {
        print!("{}", config::help());
        return;
    }
    if config.version {
        println!("xournal-rpc {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if config.init_config {
        let result = config::config_file_path()
            .ok_or_else(|| "cannot find config directory: set XDG_CONFIG_HOME or HOME".to_string())
            .and_then(|path| {
                config::init_config(&path)?;
                println!("Created {}", path.display());
                Ok(())
            });
        if let Err(error) = result {
            eprintln!("xournal-rpc: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = config.validate() {
        eprintln!("xournal-rpc: {error}");
        std::process::exit(1);
    }

    if config.verbose {
        eprintln!(
            "xournal-rpc: starting version {} (pid {}), interval {}s",
            env!("CARGO_PKG_VERSION"),
            std::process::id(),
            config.interval
        );
        if let Some(path) = config::config_file_path() {
            eprintln!(
                "xournal-rpc: configuration path: {} (exists: {})",
                path.display(),
                path.exists()
            );
        }
    }
    let mut detector = WindowDetector::new(config.verbose);

    if config.dry_run {
        let status = detect(&config, &mut detector);
        print_status(&status, &config);
        return;
    }

    let running = Arc::new(AtomicBool::new(true));
    let signal = running.clone();
    if let Err(error) = ctrlc::set_handler(move || signal.store(false, Ordering::SeqCst)) {
        eprintln!("xournal-rpc: failed to install signal handler: {error}");
    }

    let mut presence = Presence::new(&config);
    let mut active = false;
    let mut backoff = Backoff::new();

    let mut last_config_error = None;
    while running.load(Ordering::SeqCst) {
        if reload_config(&mut config, &args, &mut last_config_error) {
            presence.configure(&config);
            detector.set_verbose(config.verbose);
            backoff.reset();
        }
        let status = detect(&config, &mut detector);
        if config.verbose {
            eprintln!("xournal-rpc: detection: running={}, focused={}, window_pid={:?}, started={:?}, document={:?}", status.running, status.focused, status.pid, status.started, status.document);
            if !status.running {
                eprintln!("xournal-rpc: waiting for Xournal++ to open");
            } else if !should_publish(&status, &config) {
                eprintln!("xournal-rpc: activity hidden: waiting for Xournal++ focus (--always disables this)");
            }
        }

        if should_publish(&status, &config) {
            if presence.set(&status) {
                active = true;
                backoff.reset();
                if config.once {
                    break;
                }
                sleep_interruptible(Duration::from_secs(config.interval), &running);
            } else {
                active = false;
                if config.once {
                    std::process::exit(1);
                }
                let delay = backoff.next();
                if config.verbose {
                    eprintln!("xournal-rpc: retrying in {}s", delay.as_secs());
                }
                sleep_interruptible(delay, &running);
            }
        } else {
            if active {
                presence.clear();
                active = false;
            }
            if config.once {
                break;
            }
            sleep_interruptible(Duration::from_secs(config.interval), &running);
        }
    }

    if !config.once {
        presence.clear();
    }
    presence.close();
    if config.verbose {
        eprintln!("xournal-rpc: stopped");
    }
}

fn detect(config: &Config, detector: &mut WindowDetector) -> Status {
    let window = detector.focused_xournal();
    let focused = window.as_ref().is_some_and(|window| window.focused);
    let pid = window.as_ref().and_then(|window| window.pid);
    let started = process::find_start(pid, config.verbose);
    let running = started.is_some() || window.is_some();
    let document = if config.show_document {
        window.and_then(|window| window::clean_title(&window.title))
    } else {
        None
    };

    Status {
        running,
        focused,
        document,
        started,
        pid,
    }
}

fn should_publish(status: &Status, config: &Config) -> bool {
    status.running && (!config.only_when_focused || status.focused)
}

fn reload_config(config: &mut Config, args: &[String], last_error: &mut Option<String>) -> bool {
    let next = Config::from_args(args.iter().cloned()).and_then(|next| {
        next.validate()?;
        Ok(next)
    });
    match next {
        Ok(next) => {
            *last_error = None;
            if *config == next {
                return false;
            }
            *config = next;
            if config.verbose {
                eprintln!("xournal-rpc: configuration reloaded");
            }
            true
        }
        Err(error) => {
            if last_error.as_ref() != Some(&error) {
                eprintln!("xournal-rpc: configuration reload failed; keeping last valid settings: {error}");
                *last_error = Some(error);
            }
            false
        }
    }
}

fn print_status(status: &Status, config: &Config) {
    println!("running: {}", if status.running { "yes" } else { "no" });
    println!("focused: {}", if status.focused { "yes" } else { "no" });
    println!(
        "activity visible: {}",
        if should_publish(status, config) {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "document sharing: {}",
        if config.show_document {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "timer: {}",
        if config.show_timer {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "codeberg button: {}",
        if config.show_codeberg {
            "enabled"
        } else {
            "disabled"
        }
    );
    if let Some(pid) = status.pid {
        println!("pid: {pid}");
    }
    // Use the same builder as the live presence so the preview matches its payload.
    println!(
        "activity: {}",
        serde_json::to_string_pretty(&discord::build_activity(config, status))
            .expect("activity is serializable")
    );
}

fn sleep_interruptible(duration: Duration, running: &AtomicBool) {
    let step = Duration::from_millis(200);
    let mut remaining = duration;
    while remaining > Duration::ZERO && running.load(Ordering::SeqCst) {
        let chunk = remaining.min(step);
        sleep(chunk);
        remaining -= chunk;
    }
}

struct Backoff {
    current: Duration,
    max: Duration,
}

impl Backoff {
    fn new() -> Self {
        Self {
            current: Duration::from_secs(2),
            max: Duration::from_secs(60),
        }
    }

    fn next(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * 2).min(self.max);
        delay
    }

    fn reset(&mut self) {
        self.current = Duration::from_secs(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_hiding_is_opt_in_and_requires_confirmed_focus() {
        let mut status = Status {
            running: true,
            focused: false,
            document: None,
            started: Some(123),
            pid: Some(42),
        };
        let mut config = Config::default();
        assert!(should_publish(&status, &config));
        config.only_when_focused = true;
        assert!(!should_publish(&status, &config));
        status.focused = true;
        assert!(should_publish(&status, &config));
        status.running = false;
        assert!(!should_publish(&status, &config));
        config.only_when_focused = false;
        assert!(!should_publish(&status, &config));
    }
}
