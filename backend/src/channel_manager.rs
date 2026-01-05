use crate::models::{Channel, ChannelId, Channels, Participant, ParticipantId};
use futures::channel::mpsc::Sender;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

#[cfg(test)]
#[path = "channel_manager_tests.rs"]
mod channel_manager_tests;

#[derive(Clone)]
pub struct ChannelManager {
    pub channels: Channels,
    next_participant_id: Arc<AtomicUsize>, // To generate unique IDs for participants
}

impl ChannelManager {
    pub fn new() -> Self {
        ChannelManager {
            channels: Arc::new(RwLock::new(HashMap::new())),
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
        let mut channels = self.channels.write().await;

        // If the channel doesn't exist yet and we're at the max channel limit, reject.
        if !channels.contains_key(&channel_id) && channels.len() >= *max_channel_count {
            warn!(
                "Rejected new channel creation: Max channels limit ({}) reached.",
                *max_channel_count
            );
            return None;
        }

        // Get the channel or insert a new one wrapped in Arc<Mutex<...>>
        let channel_mutex = channels
            .entry(channel_id)
            .or_insert_with(|| Arc::new(Mutex::new(Channel::new())))
            .clone();

        // Release the global write lock on the map immediately
        drop(channels);

        // Lock the specific channel
        let mut channel = channel_mutex.lock().await;

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
        // First, acquire a read lock to find the channel
        let channels_read = self.channels.read().await;
        let channel_mutex_opt = channels_read.get(channel_id).cloned();
        drop(channels_read); // Release read lock

        if let Some(channel_mutex) = channel_mutex_opt {
            let mut channel = channel_mutex.lock().await;
            if channel.remove_participant_by_id(participant_id) {
                info!(
                    "Participant {} removed from channel {}.",
                    participant_id, channel_id
                );
            }
            let is_empty = channel.is_empty();
            drop(channel); // Release channel lock

            // If the channel is empty, acquire write lock to remove it
            if is_empty {
                let mut channels_write = self.channels.write().await;
                // Double check if it's still empty and exists (race condition check)
                if let Some(c) = channels_write.get(channel_id) {
                    let c_lock = c.lock().await;
                    if c_lock.is_empty() {
                        drop(c_lock); // Must drop before removing to avoid any weirdness, though removing drops the Arc which is fine
                        info!("Channel {} is empty and removed.", channel_id);
                        channels_write.remove(channel_id);
                    }
                }
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
        // Acquire read lock to get the channel
        let channels_read = self.channels.read().await;
        let channel_mutex_opt = channels_read.get(channel_id).cloned();
        drop(channels_read); // Release read lock immediately

        if let Some(channel_mutex) = channel_mutex_opt {
            // Lock the specific channel. This is where we might block if another participant is sending,
            // but we are NOT holding the global map lock.
            let mut channel = channel_mutex.lock().await;
            channel
                .broadcast_message(message, from_participant_id)
                .await;
            let is_empty = channel.is_empty();
            drop(channel); // Release channel lock

            // After broadcasting and cleanup, check if the channel became empty
            if is_empty {
                let mut channels_write = self.channels.write().await;
                // Double check
                if let Some(c) = channels_write.get(channel_id) {
                    let c_lock = c.lock().await;
                    if c_lock.is_empty() {
                        drop(c_lock);
                        info!(
                            "Channel {} became empty after broadcast cleanup and was removed.",
                            channel_id
                        );
                        channels_write.remove(channel_id);
                    }
                }
            }
        }
    }
}
