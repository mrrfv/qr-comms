use crate::models::{Channel, ChannelId, Channels, Participant, ParticipantId};
use futures::channel::mpsc::Sender;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct ChannelManager {
    pub channels: Channels,
    next_participant_id: Arc<AtomicUsize>, // To generate unique IDs for participants
}

impl ChannelManager {
    pub fn new() -> Self {
        ChannelManager {
            channels: Arc::new(Mutex::new(HashMap::new())),
            next_participant_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Generates a new unique participant ID.
    fn get_next_participant_id(&self) -> ParticipantId {
        self.next_participant_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Adds a participant to a specific channel. If the channel does not exist, it creates it.
    /// Returns `Some(participant_id)` if the participant was added, `None` if the channel is full.
    pub async fn add_participant_to_channel(
        &self,
        channel_id: ChannelId,
        participant_sender: Sender<axum::extract::ws::Message>,
        max_channel_count: &usize,
    ) -> Option<ParticipantId> {
        let mut channels = self.channels.lock().await;

        // If the channel doesn't exist yet and we're at the max channel limit, reject.
        if !channels.contains_key(&channel_id) && channels.len() >= *max_channel_count {
            warn!(
                "Rejected new channel creation: Max channels limit ({}) reached.",
                *max_channel_count
            );
            return None;
        }

        let channel = channels.entry(channel_id).or_insert_with(Channel::new);

        let new_participant_id = self.get_next_participant_id();
        let participant = Participant::new(new_participant_id, participant_sender);

        if channel.add_participant(participant) {
            info!(
                "Participant {} added to channel {}.",
                new_participant_id, channel_id
            );
            Some(new_participant_id)
        } else {
            debug!(
                "Channel {} is full. Participant {} not added.",
                channel_id, new_participant_id
            );
            None // Channel is full (i.e., has 2 participants)
        }
    }

    /// Removes a participant from a specific channel using their participant ID.
    /// If the channel becomes empty after removal, it is also removed from the manager.
    pub async fn remove_participant_from_channel(
        &self,
        channel_id: &ChannelId,
        participant_id: ParticipantId,
    ) {
        let mut channels = self.channels.lock().await;
        if let Some(channel) = channels.get_mut(channel_id) {
            if channel.remove_participant_by_id(participant_id) {
                info!(
                    "Participant {} removed from channel {}.",
                    participant_id, channel_id
                );
            }
            // If the channel is empty after removal, remove the channel itself
            if channel.is_empty() {
                info!("Channel {} is empty and removed.", channel_id);
                channels.remove(channel_id);
            }
        }
    }

    /// Broadcasts a message to all participants in a channel, excluding the sender.
    pub async fn broadcast_message_in_channel(
        &self,
        channel_id: &ChannelId,
        message: axum::extract::ws::Message,
        from_participant_id: ParticipantId,
    ) {
        let mut channels = self.channels.lock().await;
        if let Some(channel) = channels.get_mut(channel_id) {
            channel
                .broadcast_message(message, from_participant_id)
                .await;
            // After broadcasting and cleanup, check if the channel became empty
            if channel.is_empty() {
                info!(
                    "Channel {} became empty after broadcast cleanup and was removed.",
                    channel_id
                );
                channels.remove(channel_id);
            }
        }
    }
}
