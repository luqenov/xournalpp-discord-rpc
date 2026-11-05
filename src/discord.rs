use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

use crate::config::Config;
use crate::Status;

pub struct Presence {
    client: Option<DiscordIpcClient>,
    client_id: String,
    verbose: bool,
}

impl Presence {
    pub fn new(config: &Config) -> Self {
        Self {
            client: None,
            client_id: config.client_id.clone(),
            verbose: config.verbose,
        }
    }

    pub fn set(&mut self, status: &Status) -> bool {
        if self.client.is_none() {
            match self.connect() {
                Some(client) => self.client = Some(client),
                None => return false,
            }
        }

        let mut activity = activity::Activity::new()
            .details("Studying")
            .assets(
                activity::Assets::new()
                    .large_image("xournalpp")
                    .large_text("Xournal++"),
            )
            .buttons(vec![activity::Button::new(
                "GitHub",
                "https://github.com/luqenov/xournalpp-discord-rpc",
            )]);

        if let Some(document) = &status.document {
            activity = activity.state(document);
        }
        if let Some(started) = status.started {
            activity = activity.timestamps(activity::Timestamps::new().start(started));
        }

        let client = self.client.as_mut().expect("client was connected");
        match client.set_activity(activity) {
            Ok(()) => {
                if self.verbose {
                    eprintln!("xournal-rpc: presence updated");
                }
                true
            }
            Err(error) => {
                if self.verbose {
                    eprintln!("xournal-rpc: failed to update presence: {error}");
                }
                self.client = None;
                false
            }
        }
    }

    pub fn clear(&mut self) {
        if let Some(client) = self.client.as_mut() {
            if let Err(error) = client.clear_activity() {
                if self.verbose {
                    eprintln!("xournal-rpc: failed to clear presence: {error}");
                }
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.close();
        }
    }

    fn connect(&mut self) -> Option<DiscordIpcClient> {
        let mut client = DiscordIpcClient::new(&self.client_id);
        match client.connect() {
            Ok(()) => {
                if self.verbose {
                    eprintln!("xournal-rpc: connected to Discord");
                }
                Some(client)
            }
            Err(error) => {
                if self.verbose {
                    eprintln!("xournal-rpc: Discord connection failed: {error}");
                }
                None
            }
        }
    }
}
