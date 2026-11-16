use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static OUTPUT: Mutex<()> = Mutex::new(());

pub fn diagnostic(message: fmt::Arguments<'_>) {
    if !ACTIVE.load(Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

pub fn warning(message: fmt::Arguments<'_>) {
    let _guard = OUTPUT.lock().unwrap();
    if ACTIVE.load(Ordering::Relaxed) {
        eprint!("\r\x1b[2K");
    }
    eprintln!("{message}");
}

pub struct Terminal {
    status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Terminal {
    pub fn new(enabled: bool) -> Self {
        let status = Arc::new(Mutex::new("Starting…".to_string()));
        let stop = Arc::new(AtomicBool::new(false));
        let enabled = enabled
            && io::stderr().is_terminal()
            && std::env::var("TERM").as_deref() != Ok("dumb")
            && std::env::var_os("CI").is_none();
        let worker = enabled.then(|| {
            ACTIVE.store(true, Ordering::Relaxed);
            eprintln!("Xournal++ · Discord RPC {}", env!("CARGO_PKG_VERSION"));
            let status = status.clone();
            let stop = stop.clone();
            thread::spawn(move || {
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let mut index = 0;
                while !stop.load(Ordering::Relaxed) {
                    {
                        let _guard = OUTPUT.lock().unwrap();
                        let text =
                            compact(&status.lock().unwrap(), terminal_width().saturating_sub(4));
                        let mut stderr = io::stderr().lock();
                        let _ = write!(stderr, "\r\x1b[2K{} {text}", frames[index]);
                        let _ = stderr.flush();
                    }
                    index = (index + 1) % frames.len();
                    thread::sleep(Duration::from_millis(100));
                }
            })
        });
        Self {
            status,
            stop,
            worker,
        }
    }

    pub fn set(&self, text: impl Into<String>) {
        *self.status.lock().unwrap() = text.into();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
            let _guard = OUTPUT.lock().unwrap();
            eprintln!("\r\x1b[2K✓ Stopped");
            ACTIVE.store(false, Ordering::Relaxed);
        }
    }
}

fn terminal_width() -> usize {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut size) } == 0
        && size.ws_col > 0
    {
        usize::from(size.ws_col)
    } else {
        80
    }
}

fn compact(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut used = 0;
    for character in text.chars().filter(|character| !character.is_control()) {
        // NOTE: Reserve two columns for non-ASCII characters to avoid line wrapping.
        let columns = if character.is_ascii() { 1 } else { 2 };
        if used + columns + 2 > width {
            if width >= 2 {
                result.push('…');
            }
            break;
        }
        result.push(character);
        used += columns;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_text_cannot_inject_escape_sequences_or_wrap() {
        assert_eq!(compact("notes\n\r\t.pdf", 40), "notes.pdf");
        assert_eq!(compact("abcdef", 5), "abc…");
        assert_eq!(compact("ááá", 6), "áá…");
        assert_eq!(compact("abc", 0), "");
    }
}
