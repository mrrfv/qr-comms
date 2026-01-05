use axum::extract::ws::Message;
use futures::SinkExt;
use futures::channel::mpsc::Sender;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

pub type ChannelId = Uuid;
pub type ParticipantId = usize;

// Represents app configuration.
#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub server_port: u16,
    pub max_channel_count: usize,
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,
    #[serde(default = "default_rate_limit_per_second")]
    pub rate_limit_per_second: u32,
}

fn default_max_message_size() -> usize {
    64 * 1024 // 64 KB
}

fn default_rate_limit_burst() -> u32 {
    10
}

fn default_rate_limit_per_second() -> u32 {
    5
}

/// Represents a single participant in a channel.
#[derive(Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub sender: Sender<Message>,
}

impl Participant {
    pub fn new(id: ParticipantId, sender: Sender<Message>) -> Self {
        Participant { id, sender }
    }
}

/// Represents a single communication channel
#[derive(Debug)]
pub struct Channel {
    pub participants: Vec<Participant>,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            participants: Vec::with_capacity(2), // Pre-allocate capacity for 2 participants
        }
    }

    /// Tries to add a participant to the channel.
    /// Returns true if added, false if the channel is full.
    pub fn add_participant(&mut self, participant: Participant) -> bool {
        if self.participants.len() < 2 {
            self.participants.push(participant);
            true
        } else {
            false // Channel is full
        }
    }

    /// Removes a participant from the channel based on their unique ID.
    /// Returns true if a participant was removed.
    pub fn remove_participant_by_id(&mut self, participant_id: ParticipantId) -> bool {
        let initial_len = self.participants.len();
        self.participants.retain(|p| p.id != participant_id);
        self.participants.len() < initial_len
    }

    /// Sends a message to all participants in this channel, excluding the sender of the message.
    /// Cleans up disconnected participants during the process.
    pub async fn broadcast_message(
        &mut self,
        message: Message,
        from_participant_id: ParticipantId,
    ) {
        let mut disconnected_ids = Vec::new();

        for participant in &mut self.participants {
            // Only send to others, not back to the original sender
            if participant.id != from_participant_id {
                if let Err(e) = participant.sender.send(message.clone()).await {
                    error!(
                        "Error sending message to participant {}: {}",
                        participant.id, e
                    );
                    // If sending fails, assume the participant is disconnected and mark them for removal.
                    disconnected_ids.push(participant.id);
                }
            }
        }

        // Clean up disconnected participants
        // Iterate in reverse to avoid index shifts if multiple removals occur
        for &id_to_remove in disconnected_ids.iter().rev() {
            self.remove_participant_by_id(id_to_remove);
            info!(
                "Cleaned up disconnected participant {} from channel.",
                id_to_remove
            );
        }
    }

    /// Checks if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }
}

/// Manages all active communication channels.
pub type Channels = Arc<Mutex<HashMap<ChannelId, Channel>>>;
