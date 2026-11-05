use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    pub pid: Option<u32>,
    pub focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Niri,
    Hyprland,
    Sway,
    X11,
}

impl Backend {
    fn detect() -> Option<Self> {
        if env::var_os("NIRI_SOCKET").is_some() {
            return Some(Self::Niri);
        }
        if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            return Some(Self::Hyprland);
        }
        if env::var_os("SWAYSOCK").is_some() || env::var_os("I3SOCK").is_some() {
            return Some(Self::Sway);
        }
        if env::var_os("DISPLAY").is_some() {
            return Some(Self::X11);
        }
        None
    }

    fn all() -> [Self; 4] {
        [Self::Niri, Self::Hyprland, Self::Sway, Self::X11]
    }

    fn windows(self) -> Result<Vec<WindowInfo>, String> {
        match self {
            Self::Niri => niri(),
            Self::Hyprland => hyprland(),
            Self::Sway => sway(),
            Self::X11 => x11(),
        }
    }
}

pub struct WindowDetector {
    backend: Option<Backend>,
    verbose: bool,
}

impl WindowDetector {
    pub fn new(verbose: bool) -> Self {
        Self {
            backend: Backend::detect(),
            verbose,
        }
    }

    pub fn focused_xournal(&mut self) -> Option<WindowInfo> {
        if let Some(backend) = self.backend {
            match backend.windows() {
                Ok(windows) => return pick(windows),
                Err(error) => {
                    if self.verbose {
                        eprintln!("xournal-rpc: {backend:?} backend unavailable: {error}");
                    }
                    self.backend = None;
                }
            }
        }

        for backend in Backend::all() {
            if Some(backend) == self.backend {
                continue;
            }
            match backend.windows() {
                Ok(windows) => {
                    self.backend = Some(backend);
                    return pick(windows);
                }
                Err(error) => {
                    if self.verbose {
                        eprintln!("xournal-rpc: {backend:?} backend unavailable: {error}");
                    }
                }
            }
        }

        None
    }
}

fn pick(windows: Vec<WindowInfo>) -> Option<WindowInfo> {
    windows
        .iter()
        .find(|window| window.focused)
        .cloned()
        .or_else(|| windows.into_iter().next())
}

fn is_xournalpp(class: &str) -> bool {
    let class = class.to_ascii_lowercase();
    class.contains("xournalpp") || class.contains("xournal++")
}

pub fn clean_title(title: &str) -> Option<String> {
    let title = title.strip_suffix(" - Xournal++").unwrap_or(title).trim();
    let title = strip_page_counter(title);
    let title = title.trim_start_matches('*').trim_start();
    let title = strip_path_prefix(title);
    let title = title.trim_start_matches('*').trim();

    if title.is_empty() || title == "Xournal++" || title == "Unsaved Document" {
        None
    } else {
        Some(title.to_string())
    }
}

fn strip_page_counter(title: &str) -> &str {
    let Some(rest) = title.strip_prefix('[') else {
        return title;
    };
    let Some((counter, rest)) = rest.split_once(']') else {
        return title;
    };
    let valid = counter.split_once('/').is_some_and(|(page, total)| {
        !page.is_empty()
            && !total.is_empty()
            && page.chars().all(|c| c.is_ascii_digit())
            && total.chars().all(|c| c.is_ascii_digit())
    });
    if valid {
        rest.trim_start()
    } else {
        title
    }
}

fn strip_path_prefix(title: &str) -> &str {
    let Some(rest) = title.strip_prefix('[') else {
        return title;
    };
    let Some((_, rest)) = rest.split_once("] - ") else {
        return title;
    };
    rest.trim_start()
}

fn niri() -> Result<Vec<WindowInfo>, String> {
    let mut command = Command::new("niri");
    command.args(["msg", "--json", "windows"]);
    if env::var_os("NIRI_SOCKET").is_none() {
        if let Some(socket) = niri_socket() {
            command.env("NIRI_SOCKET", socket);
        }
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    parse_niri(&output.stdout)
}

fn niri_socket() -> Option<PathBuf> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")?;
    let mut candidates: Vec<PathBuf> = fs::read_dir(PathBuf::from(runtime))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("niri.") && name.ends_with(".sock"))
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

#[derive(Deserialize)]
struct NiriWindow {
    title: Option<String>,
    app_id: Option<String>,
    pid: Option<u32>,
    #[serde(default)]
    is_focused: bool,
}

fn parse_niri(bytes: &[u8]) -> Result<Vec<WindowInfo>, String> {
    let windows: Vec<NiriWindow> =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    Ok(windows
        .into_iter()
        .filter(|window| window.app_id.as_deref().is_some_and(is_xournalpp))
        .map(|window| WindowInfo {
            title: window.title.unwrap_or_default(),
            pid: window.pid,
            focused: window.is_focused,
        })
        .collect())
}

fn hyprland() -> Result<Vec<WindowInfo>, String> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    parse_hyprland(&output.stdout)
}

#[derive(Deserialize)]
struct HyprWindow {
    title: Option<String>,
    class: Option<String>,
    #[serde(rename = "initialClass")]
    initial_class: Option<String>,
    pid: Option<u32>,
    #[serde(rename = "focusHistoryID")]
    focus_history_id: Option<u32>,
}

fn parse_hyprland(bytes: &[u8]) -> Result<Vec<WindowInfo>, String> {
    let windows: Vec<HyprWindow> =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    Ok(windows
        .into_iter()
        .filter(|window| {
            window.class.as_deref().is_some_and(is_xournalpp)
                || window.initial_class.as_deref().is_some_and(is_xournalpp)
        })
        .map(|window| WindowInfo {
            title: window.title.unwrap_or_default(),
            pid: window.pid,
            focused: window.focus_history_id == Some(0),
        })
        .collect())
}

fn sway() -> Result<Vec<WindowInfo>, String> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_tree", "-r"])
        .output()
        .or_else(|_| Command::new("i3-msg").args(["-t", "get_tree"]).output())
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    parse_sway(&output.stdout)
}

#[derive(Deserialize)]
struct SwayNode {
    name: Option<String>,
    app_id: Option<String>,
    pid: Option<u32>,
    #[serde(default)]
    focused: bool,
    window_properties: Option<SwayWindowProperties>,
    #[serde(default)]
    nodes: Vec<SwayNode>,
    #[serde(default)]
    floating_nodes: Vec<SwayNode>,
}

#[derive(Deserialize)]
struct SwayWindowProperties {
    class: Option<String>,
    instance: Option<String>,
    title: Option<String>,
}

fn parse_sway(bytes: &[u8]) -> Result<Vec<WindowInfo>, String> {
    let root: SwayNode = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let mut windows = Vec::new();
    collect_sway(&root, &mut windows);
    Ok(windows)
}

fn collect_sway(node: &SwayNode, windows: &mut Vec<WindowInfo>) {
    let class = node
        .app_id
        .as_deref()
        .or_else(|| {
            node.window_properties
                .as_ref()
                .and_then(|properties| properties.class.as_deref())
        })
        .or_else(|| {
            node.window_properties
                .as_ref()
                .and_then(|properties| properties.instance.as_deref())
        });

    if class.is_some_and(is_xournalpp) {
        let title = node
            .name
            .clone()
            .or_else(|| {
                node.window_properties
                    .as_ref()
                    .and_then(|properties| properties.title.clone())
            })
            .unwrap_or_default();
        windows.push(WindowInfo {
            title,
            pid: node.pid,
            focused: node.focused,
        });
    }

    for child in node.nodes.iter().chain(node.floating_nodes.iter()) {
        collect_sway(child, windows);
    }
}

fn x11() -> Result<Vec<WindowInfo>, String> {
    let active = xprop(&["-root", "_NET_ACTIVE_WINDOW"])?;
    let id = parse_active_window(&active).ok_or("no active window")?;
    let class = xprop(&["-id", &id, "WM_CLASS"])?;
    if !parse_class(&class).as_deref().is_some_and(is_xournalpp) {
        return Ok(Vec::new());
    }
    let name = xprop(&["-id", &id, "_NET_WM_NAME", "WM_NAME"])?;
    let pid = xprop(&["-id", &id, "_NET_WM_PID"])
        .ok()
        .and_then(|value| parse_pid(&value));
    Ok(vec![WindowInfo {
        title: parse_title(&name).unwrap_or_default(),
        pid,
        focused: true,
    }])
}

fn xprop(args: &[&str]) -> Result<String, String> {
    let output = Command::new("xprop")
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_active_window(output: &str) -> Option<String> {
    output
        .split('#')
        .nth(1)
        .map(|value| value.trim().to_string())
}

fn parse_class(output: &str) -> Option<String> {
    output
        .split_once('=')
        .map(|(_, value)| value.trim().to_string())
}

fn parse_title(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.split_once('=').map(|(_, value)| value.trim()))
        .map(|value| value.trim_matches('"').to_string())
}

fn parse_pid(output: &str) -> Option<u32> {
    output.split_once('=')?.1.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_titles() {
        assert_eq!(
            clean_title("notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(
            clean_title("*notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(
            clean_title("[1/3]  notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(
            clean_title("[1/3]  *notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(
            clean_title("[1/3]  *[/home/u] - notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(
            clean_title("[/home/u] - notes.pdf - Xournal++").as_deref(),
            Some("notes.pdf")
        );
        assert_eq!(clean_title("Unsaved Document - Xournal++"), None);
        assert_eq!(clean_title("Xournal++"), None);
    }

    #[test]
    fn parses_niri_windows() {
        let json = br#"[
            {"title":"notes.pdf - Xournal++","app_id":"com.github.xournalpp.xournalpp","pid":42,"is_focused":true},
            {"title":"other","app_id":"firefox","pid":7,"is_focused":false}
        ]"#;
        let windows = parse_niri(json).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].title, "notes.pdf - Xournal++");
        assert_eq!(windows[0].pid, Some(42));
        assert!(windows[0].focused);
    }

    #[test]
    fn parses_hyprland_windows() {
        let json = br#"[
            {"title":"notes.pdf - Xournal++","class":"xournalpp","initialClass":"xournalpp","pid":42,"focusHistoryID":0},
            {"title":"other","class":"firefox","initialClass":"firefox","pid":7,"focusHistoryID":1}
        ]"#;
        let windows = parse_hyprland(json).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].title, "notes.pdf - Xournal++");
        assert!(windows[0].focused);
    }

    #[test]
    fn parses_sway_tree() {
        let json = br#"{
            "name":"root","focused":false,
            "nodes":[
                {"name":"notes.pdf - Xournal++","app_id":"com.github.xournalpp.xournalpp","pid":42,"focused":true,"nodes":[]},
                {"name":"other","app_id":"firefox","pid":7,"focused":false,"nodes":[]}
            ],
            "floating_nodes":[]
        }"#;
        let windows = parse_sway(json).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].title, "notes.pdf - Xournal++");
        assert!(windows[0].focused);
    }

    #[test]
    fn parses_xprop_output() {
        assert_eq!(
            parse_active_window("_NET_ACTIVE_WINDOW(WINDOW): window id # 0x3a00007").as_deref(),
            Some("0x3a00007")
        );
        assert_eq!(
            parse_class("WM_CLASS(STRING) = \"xournalpp\", \"Xournal++\"").as_deref(),
            Some("\"xournalpp\", \"Xournal++\"")
        );
        assert_eq!(
            parse_title("_NET_WM_NAME(UTF8_STRING) = \"notes.pdf - Xournal++\"").as_deref(),
            Some("notes.pdf - Xournal++")
        );
        assert_eq!(parse_pid("_NET_WM_PID(CARDINAL) = 42"), Some(42));
    }
}
