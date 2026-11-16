use crate::ipc::IpcClient;
use discord_rich_presence::activity;

use crate::config::Config;
use crate::Status;

pub struct Presence {
    client: Option<IpcClient>,
    config: Config,
    last_error: Option<String>,
}

impl Presence {
    pub fn new(config: &Config) -> Self {
        Self {
            client: None,
            last_error: None,
            config: config.clone(),
        }
    }

    pub fn configure(&mut self, config: &Config) {
        if self.config.client_id != config.client_id {
            self.clear();
            self.close();
        }
        self.config = config.clone();
    }

    pub fn set(&mut self, status: &Status) -> bool {
        if self.client.is_none() {
            match self.connect() {
                Some(client) => self.client = Some(client),
                None => return false,
            }
        }

        let activity = build_activity(&self.config, status);

        let client = self.client.as_mut().expect("client was connected");
        match client.set_activity(serde_json::to_value(activity).expect("activity is serializable"))
        {
            Ok(()) => {
                self.last_error = None;
                if self.config.verbose {
                    crate::terminal::diagnostic(format_args!(
                        "xournal-rpc: presence update acknowledged by Discord"
                    ));
                }
                true
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                if self.config.verbose {
                    crate::terminal::diagnostic(format_args!(
                        "xournal-rpc: failed to update presence: {error}"
                    ));
                }
                self.client = None;
                false
            }
        }
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn clear(&mut self) {
        if let Some(client) = self.client.as_mut() {
            if let Err(error) = client.clear_activity() {
                if self.config.verbose {
                    crate::terminal::warning(format_args!(
                        "xournal-rpc: failed to clear presence: {error}"
                    ));
                }
                self.client = None;
            } else if self.config.verbose {
                crate::terminal::diagnostic(format_args!(
                    "xournal-rpc: activity clear acknowledged by Discord"
                ));
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.close();
        }
    }

    fn connect(&mut self) -> Option<IpcClient> {
        if self.config.verbose {
            crate::terminal::diagnostic(format_args!(
                "xournal-rpc: connecting to Discord IPC (client ID {})",
                self.config.client_id
            ));
        }
        match IpcClient::connect(&self.config.client_id, self.config.verbose) {
            Ok(client) => {
                if self.config.verbose {
                    crate::terminal::diagnostic(format_args!("xournal-rpc: connected to Discord"));
                }
                Some(client)
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                if self.config.verbose {
                    crate::terminal::diagnostic(format_args!(
                        "xournal-rpc: Discord connection failed: {error}"
                    ));
                }
                None
            }
        }
    }
}

pub fn build_activity<'a>(config: &Config, status: &'a Status) -> activity::Activity<'a> {
    let mut activity = activity::Activity::new().assets(
        activity::Assets::new()
            .large_image("xournalpp")
            .large_text("Xournal++"),
    );
    if config.show_message {
        activity = activity.details(config.message.clone());
    }
    if config.show_codeberg {
        activity = activity.buttons(vec![activity::Button::new(
            "Codeberg",
            "https://codeberg.org/luqenov/xournalpp-discord-rpc",
        )]);
    }
    if config.show_document {
        if let Some(document) = &status.document {
            activity = activity.state(document);
        }
    }
    if config.show_timer {
        if let Some(started) = status.started {
            activity = activity.timestamps(activity::Timestamps::new().start(started));
        }
    }
    activity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_respects_message_and_privacy_settings() {
        let status = Status {
            running: true,
            focused: true,
            document: Some("notes.pdf".into()),
            started: Some(123),
            pid: Some(42),
        };
        let mut config = Config::default();
        let payload = serde_json::to_value(build_activity(&config, &status)).unwrap();
        assert_eq!(payload["details"], "Studying");
        assert_eq!(payload["state"], "notes.pdf");
        assert_eq!(payload["timestamps"]["start"], 123);
        assert_eq!(payload["buttons"][0]["label"], "Codeberg");
        config.message = "Revisando matemática".into();
        config.show_document = false;
        config.show_codeberg = false;
        config.show_timer = false;
        let payload = serde_json::to_value(build_activity(&config, &status)).unwrap();
        assert_eq!(payload["details"], "Revisando matemática");
        for field in ["state", "buttons", "timestamps"] {
            assert!(payload.get(field).is_none(), "{field}");
        }
    }
}
