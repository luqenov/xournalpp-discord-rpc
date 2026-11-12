use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf, Running);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "xrpc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("xournal-rpc")).unwrap();
        fs::create_dir_all(path.join("bin")).unwrap();
        fs::write(
            path.join("bin/niri"),
            "#!/bin/sh\nexec /bin/cat \"$WINDOW_FIXTURE\"\n",
        )
        .unwrap();
        fs::set_permissions(path.join("bin/niri"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::copy("/bin/sleep", path.join("bin/xournalpp")).unwrap();
        let process = Running(
            Command::new(path.join("bin/xournalpp"))
                .arg("60")
                .spawn()
                .unwrap(),
        );
        let fixture = Self(path, process);
        fixture.focus(true);
        fixture
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xournal-rpc"));
        command
            .env_clear()
            .env("PATH", self.0.join("bin"))
            .env("XDG_CONFIG_HOME", &self.0)
            .env("XDG_RUNTIME_DIR", &self.0)
            .env("NIRI_SOCKET", "fixture")
            .env("WINDOW_FIXTURE", self.0.join("windows.json"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn config(&self, contents: &str) {
        let temporary = self.0.join("config-new");
        fs::write(&temporary, contents).unwrap();
        fs::rename(temporary, self.0.join("xournal-rpc/config.toml")).unwrap();
    }
    fn focus(&self, focused: bool) {
        let temporary = self.0.join("windows-new");
        fs::write(&temporary, json!([{"title":"notes.pdf - Xournal++", "app_id":"com.github.xournalpp.xournalpp", "pid": self.1.0.id(), "is_focused":focused}]).to_string()).unwrap();
        fs::rename(temporary, self.0.join("windows.json")).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn preview(output: Output) -> (String, Value) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let payload = serde_json::from_str(text.split_once("activity: ").unwrap().1).unwrap();
    (text, payload)
}

#[test]
fn config_environment_and_cli_precedence_match_preview() {
    let fixture = Fixture::new();
    fixture.config("message = 'from file'\nshow_timer = false\nshow_codeberg = false\nshow_document = false\nonly_when_focused = true");
    fixture.focus(false);
    let (text, payload) = preview(fixture.run(&["--dry-run"]));
    assert!(text.contains("activity visible: no"));
    assert_eq!(payload["details"], "from file");
    for field in ["timestamps", "buttons", "state"] {
        assert!(payload.get(field).is_none());
    }
    let output = fixture
        .command()
        .env("XOURNAL_RPC_MESSAGE", "from env")
        .env("XOURNAL_RPC_SHOW_TIMER", "true")
        .args(["--dry-run"])
        .output()
        .unwrap();
    let (_, payload) = preview(output);
    assert_eq!(payload["details"], "from env");
    assert!(payload.get("timestamps").is_some());
    let output = fixture
        .command()
        .env("XOURNAL_RPC_MESSAGE", "from env")
        .env("XOURNAL_RPC_SHOW_TIMER", "true")
        .args([
            "--dry-run",
            "--message",
            "from cli",
            "--no-timer",
            "--codeberg",
            "--document",
            "--always",
        ])
        .output()
        .unwrap();
    let (text, payload) = preview(output);
    assert!(text.contains("activity visible: yes"));
    assert_eq!(payload["details"], "from cli");
    assert!(payload.get("timestamps").is_none());
    assert_eq!(payload["state"], "notes.pdf");
    assert!(payload.get("buttons").is_some());
    // A flag-shaped message is a value, not an informational flag.
    let (_, payload) = preview(fixture.run(&["--dry-run", "--message", "--help"]));
    assert_eq!(payload["details"], "--help");
    assert!(payload.get("buttons").is_none());
}

#[test]
fn errors_are_actionable_and_init_never_overwrites() {
    let fixture = Fixture::new();
    assert!(fixture.run(&["--init-config"]).status.success());
    let path = fixture.0.join("xournal-rpc/config.toml");
    let original = fs::read_to_string(&path).unwrap();
    let output = fixture.run(&["--init-config"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("never overwritten"));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    for bad in [
        "message = [",
        "show_timer = 'false'",
        "only_when_focussed = true",
    ] {
        fixture.config(bad);
        let output = fixture.run(&["--dry-run"]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("config.toml"));
        assert!(fixture.run(&["--help"]).status.success());
        assert!(fixture.run(&["--version"]).status.success());
    }
    fixture.config("");
    for args in [
        &["--message"][..],
        &["--message="][..],
        &["--message=x"][..],
        &["--interval=1"][..],
        &["--unknown"][..],
    ] {
        let mut command_args = vec!["--dry-run"];
        command_args.extend_from_slice(args);
        assert!(!fixture.run(&command_args).status.success(), "{args:?}");
    }
    assert!(!fixture
        .command()
        .env("XOURNAL_RPC_SHOW_TIMER", "maybe")
        .arg("--dry-run")
        .output()
        .unwrap()
        .status
        .success());
}

fn receive(stream: &mut UnixStream) -> (u32, Value) {
    let mut header = [0; 8];
    stream.read_exact(&mut header).unwrap();
    let opcode = u32::from_le_bytes(header[..4].try_into().unwrap());
    let length = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
    assert!(length < 65536);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    (opcode, serde_json::from_slice(&body).unwrap())
}
fn connect(listener: &UnixListener) -> UnixStream {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => panic!("accept: {error}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    assert_eq!(receive(&mut stream).0, 0);
    stream.write_all(&1u32.to_le_bytes()).unwrap();
    stream.write_all(&2u32.to_le_bytes()).unwrap();
    stream.write_all(b"{}").unwrap();
    stream
}
fn activity(stream: &mut UnixStream) -> Value {
    let (opcode, payload) = receive(stream);
    assert_eq!(opcode, 1);
    assert_eq!(payload["cmd"], "SET_ACTIVITY");
    payload["args"]["activity"].clone()
}

#[test]
fn live_reload_keeps_valid_settings_and_tracks_focus() {
    let fixture = Fixture::new();
    let config = |message: &str| {
        format!("interval = 2\nmessage = '{message}'\nshow_timer = false\nonly_when_focused = true")
    };
    fixture.config(&config("first message"));
    let listener = UnixListener::bind(fixture.0.join("discord-ipc-0")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = Running(
        fixture
            .command()
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut stream = connect(&listener);
    assert_eq!(activity(&mut stream)["details"], "first message");
    fixture.config(&config("updated message"));
    assert_eq!(activity(&mut stream)["details"], "updated message");
    fixture.config("message = [");
    assert_eq!(activity(&mut stream)["details"], "updated message");
    fixture.config(&config("recovered message"));
    assert_eq!(activity(&mut stream)["details"], "recovered message");
    fixture.focus(false);
    assert!(activity(&mut stream).is_null());
    fixture.focus(true);
    assert_eq!(activity(&mut stream)["details"], "recovered message");
    fixture.focus(false);
    fixture.config("interval = 2\nmessage = 'background allowed'\n");
    assert_eq!(activity(&mut stream)["details"], "background allowed");
    // Changing the application ID clears the old activity and reconnects.
    fixture.config("interval = 2\nclient_id = '1561027274977972368'\nmessage = 'new application'");
    assert!(activity(&mut stream).is_null());
    assert_eq!(receive(&mut stream).0, 2);
    let mut stream = connect(&listener);
    assert_eq!(activity(&mut stream)["details"], "new application");
    // Verify graceful shutdown clears the current activity.
    unsafe {
        libc::kill(child.0.id() as i32, libc::SIGTERM);
    }
    assert!(activity(&mut stream).is_null());
    assert!(child.0.wait().unwrap().success());
}
