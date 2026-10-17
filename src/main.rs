use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use serde_json::Value;
use std::{fs, path::Path, process::Command, thread::sleep, time::Duration};

const DEFAULT_CLIENT_ID: &str = "1561027274977972367";
const APP_ID: &str = "com.github.xournalpp.xournalpp";

fn start_time(dir: &Path, btime: i64) -> Option<i64> {
    let comm = fs::read_to_string(dir.join("comm")).ok()?;
    if !comm.trim().starts_with("xournalpp") {
        return None;
    }
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    let rest = stat.rsplit_once(')')?.1;
    let ticks: i64 = rest.split_whitespace().nth(19)?.parse().ok()?;
    Some(btime + ticks / 100)
}

fn find_start() -> Option<i64> {
    let btime = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("btime ")?.trim().parse::<i64>().ok())?;
    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Some(ts) = start_time(&entry.path(), btime) {
            return Some(ts);
        }
    }
    None
}

fn doc_name() -> Option<String> {
    let out = Command::new("niri")
        .args(["msg", "--json", "windows"])
        .output()
        .ok()?;
    let windows: Value = serde_json::from_slice(&out.stdout).ok()?;
    for w in windows.as_array()? {
        if w["app_id"].as_str() == Some(APP_ID) {
            let title = w["title"].as_str().unwrap_or("");
            let name = title
                .strip_suffix(" - Xournal++")
                .unwrap_or(title)
                .trim_start_matches('*')
                .trim();
            if !name.is_empty() && name != "Xournal++" {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn connect(id: &str) -> DiscordIpcClient {
    loop {
        if let Ok(mut client) = DiscordIpcClient::new(id) {
            if client.connect().is_ok() {
                return client;
            }
        }
        sleep(Duration::from_secs(10));
    }
}

fn main() {
    let id = client_id();
    let mut client = connect(&id);
    let mut active = false;

    loop {
        let ok = match find_start() {
            Some(ts) => {
                let details = match doc_name() {
                    Some(d) => format!("Studying ({d})"),
                    None => "Studying".to_string(),
                };
                let act = activity::Activity::new()
                    .details(&details)
                    .timestamps(activity::Timestamps::new().start(ts))
                    .assets(
                        activity::Assets::new()
                            .large_image("xournalpp")
                            .large_text("Xournal++"),
                    );
                active = true;
                client.set_activity(act).is_ok()
            }
            None if active => {
                active = false;
                client.clear_activity().is_ok()
            }
            None => true,
        };

        if !ok {
            client = connect(&id);
        }
        sleep(Duration::from_secs(15));
    }
}

fn client_id() -> String {
    if let Ok(v) = std::env::var("XOURNAL_RPC_CLIENT_ID") {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let path = Path::new(&home).join(".config/xournal-rpc/client_id");
        if let Ok(v) = fs::read_to_string(path) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    DEFAULT_CLIENT_ID.to_string()
}
