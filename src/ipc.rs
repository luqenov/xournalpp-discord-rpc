use serde_json::{json, Value};
use std::env;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_FRAME: usize = 1024 * 1024;

pub struct IpcClient {
    stream: UnixStream,
    nonce: u64,
}

impl IpcClient {
    pub fn connect(client_id: &str, verbose: bool) -> Result<Self, String> {
        let paths = socket_paths();
        let mut errors = Vec::new();
        for path in paths.iter().filter(|path| path.exists()) {
            if verbose {
                crate::terminal::diagnostic(format_args!(
                    "xournal-rpc: trying IPC socket {} (5s response timeout)",
                    path.display()
                ));
            }
            let result = (|| {
                let stream = UnixStream::connect(path).map_err(|error| error.to_string())?;
                stream
                    .set_write_timeout(Some(TIMEOUT))
                    .map_err(|error| error.to_string())?;
                let mut client = Self { stream, nonce: 0 };
                let deadline = Instant::now() + TIMEOUT;
                client.send(0, &json!({"v": 1, "client_id": client_id}))?;
                let response = client.response(deadline)?;
                if response["evt"] != "READY" {
                    return Err(format!("expected READY handshake, received {response}"));
                }
                Ok(client)
            })();
            match result {
                Ok(client) => return Ok(client),
                Err(error) => {
                    if verbose {
                        crate::terminal::diagnostic(format_args!(
                            "xournal-rpc: IPC socket {} failed: {error}",
                            path.display()
                        ));
                    }
                    errors.push(format!("{}: {error}", path.display()));
                }
            }
        }
        if errors.is_empty() {
            let mut roots = Vec::new();
            for parent in paths.iter().filter_map(|path| path.parent()) {
                let root = parent.display().to_string();
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
            Err(format!("no Discord IPC socket found; searched: {}. Open Discord/Equibop in the same user session; check XDG_RUNTIME_DIR and sandbox permissions", roots.join(", ")))
        } else {
            Err(errors.join("; "))
        }
    }

    pub fn set_activity(&mut self, activity: Value) -> Result<(), String> {
        self.nonce += 1;
        let nonce = self.nonce.to_string();
        let deadline = Instant::now() + TIMEOUT;
        self.send(
            1,
            &json!({"cmd": "SET_ACTIVITY", "args": {
            "pid": std::process::id(), "activity": activity
        }, "nonce": nonce}),
        )?;
        loop {
            let response = self.response(deadline)?;
            if response["nonce"] == nonce && response["cmd"] == "SET_ACTIVITY" {
                return Ok(());
            }
        }
    }

    pub fn clear_activity(&mut self) -> Result<(), String> {
        self.set_activity(Value::Null)
    }

    pub fn close(&mut self) -> Result<(), String> {
        let result = self.send(2, &json!({}));
        let _ = self.stream.shutdown(Shutdown::Both);
        result
    }

    fn send(&mut self, opcode: u32, payload: &Value) -> Result<(), String> {
        let body = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
        self.write_frame(opcode, &body)
    }

    fn write_frame(&mut self, opcode: u32, body: &[u8]) -> Result<(), String> {
        self.stream
            .write_all(&opcode.to_le_bytes())
            .and_then(|_| self.stream.write_all(&(body.len() as u32).to_le_bytes()))
            .and_then(|_| self.stream.write_all(body))
            .map_err(|error| format!("IPC write failed: {error}"))
    }

    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> Result<(), String> {
        let mut offset = 0;
        while offset < buffer.len() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|remaining| !remaining.is_zero())
                .ok_or("IPC response timed out after 5s")?;
            self.stream
                .set_read_timeout(Some(remaining))
                .map_err(|error| error.to_string())?;
            match self.stream.read(&mut buffer[offset..]) {
                Ok(0) => return Err("IPC connection closed by Discord".into()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err("IPC response timed out after 5s".into())
                }
                Err(error) => return Err(format!("IPC read failed: {error}")),
            }
        }
        Ok(())
    }

    fn response(&mut self, deadline: Instant) -> Result<Value, String> {
        loop {
            let mut header = [0; 8];
            self.read(&mut header, deadline)?;
            let opcode = u32::from_le_bytes(header[..4].try_into().unwrap());
            let length = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
            if length > MAX_FRAME {
                return Err(format!("IPC frame exceeds {MAX_FRAME} bytes"));
            }
            let mut body = vec![0; length];
            self.read(&mut body, deadline)?;
            if opcode == 3 {
                self.write_frame(4, &body)?;
                continue;
            }
            if opcode == 4 {
                continue;
            }
            let payload: Value = serde_json::from_slice(&body)
                .map_err(|error| format!("invalid IPC JSON: {error}"))?;
            if opcode == 2 || payload["evt"] == "ERROR" {
                return Err(format!("Discord rejected RPC: {payload}"));
            }
            if opcode != 1 {
                return Err(format!("unexpected IPC opcode: {opcode}"));
            }
            return Ok(payload);
        }
    }
}

fn socket_paths() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for key in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
        if let Some(root) = env::var_os(key).filter(|value| !value.is_empty()) {
            let mut root = PathBuf::from(root);
            if key == "XDG_RUNTIME_DIR" && env::var_os("SNAP").is_some() {
                root.pop();
            }
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    // NOTE: Desktop launchers and user services may omit XDG_RUNTIME_DIR.
    let runtime = PathBuf::from(format!("/run/user/{}", unsafe { libc::geteuid() }));
    if !roots.contains(&runtime) {
        roots.push(runtime);
    }
    if !roots.contains(&PathBuf::from("/tmp")) {
        roots.push(PathBuf::from("/tmp"));
    }
    let mut paths = Vec::new();
    for root in roots {
        for subpath in [
            "",
            "app/com.discordapp.Discord",
            "app/dev.vencord.Vesktop",
            ".flatpak/com.discordapp.Discord/xdg-run",
            ".flatpak/dev.vencord.Vesktop/xdg-run",
            "snap.discord-canary",
            "snap.discord",
        ] {
            for index in 0..10 {
                paths.push(root.join(subpath).join(format!("discord-ipc-{index}")));
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (IpcClient, IpcClient) {
        let (first, second) = UnixStream::pair().unwrap();
        (
            IpcClient {
                stream: first,
                nonce: 0,
            },
            IpcClient {
                stream: second,
                nonce: 0,
            },
        )
    }

    #[test]
    fn silent_peer_times_out_and_oversized_frames_are_rejected() {
        let (mut client, mut peer) = pair();
        let start = Instant::now();
        let error = client
            .response(start + Duration::from_millis(30))
            .unwrap_err();
        assert!(error.contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(1));
        peer.stream.write_all(&1u32.to_le_bytes()).unwrap();
        peer.stream
            .write_all(&((MAX_FRAME + 1) as u32).to_le_bytes())
            .unwrap();
        assert!(client
            .response(Instant::now() + TIMEOUT)
            .unwrap_err()
            .contains("exceeds"));
    }

    #[test]
    fn close_error_is_reported_with_reason() {
        let (mut client, mut peer) = pair();
        peer.send(2, &json!({"code":4000,"message":"Invalid Client ID"}))
            .unwrap();
        let error = client.response(Instant::now() + TIMEOUT).unwrap_err();
        assert!(error.contains("Invalid Client ID"));
    }

    #[test]
    fn command_ignores_unrelated_events_until_matching_acknowledgement() {
        let (mut client, mut peer) = pair();
        let server = std::thread::spawn(move || {
            let request = peer.response(Instant::now() + TIMEOUT).unwrap();
            peer.send(1, &json!({"cmd":"DISPATCH", "evt":"CURRENT_USER_UPDATE"}))
                .unwrap();
            peer.send(1, &json!({"cmd":"SET_ACTIVITY", "nonce":"unrelated"}))
                .unwrap();
            peer.send(1, &json!({"cmd":"SET_ACTIVITY", "nonce":request["nonce"]}))
                .unwrap();
        });
        client.set_activity(json!({"details":"Studying"})).unwrap();
        server.join().unwrap();
    }
}
