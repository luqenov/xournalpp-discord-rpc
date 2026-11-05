mod config;
mod discord;
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
    pub document: Option<String>,
    pub started: Option<i64>,
    pub pid: Option<u32>,
}

fn main() {
    let config = match Config::from_args(std::env::args().skip(1)) {
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
    if let Err(error) = config.validate() {
        eprintln!("xournal-rpc: {error}");
        std::process::exit(1);
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

    while running.load(Ordering::SeqCst) {
        let status = detect(&config, &mut detector);

        if status.running {
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
}

fn detect(config: &Config, detector: &mut WindowDetector) -> Status {
    let window = detector.focused_xournal();
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
        document,
        started,
        pid,
    }
}

fn print_status(status: &Status, config: &Config) {
    println!("running: {}", if status.running { "yes" } else { "no" });
    if let Some(document) = &status.document {
        println!("document: {document}");
    }
    if let Some(pid) = status.pid {
        println!("pid: {pid}");
    }
    if let Some(started) = status.started {
        println!("started: {started}");
    }
    if !config.show_document {
        println!("document sharing: disabled");
    }
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
