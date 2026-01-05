use axum::{
    extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use tracing::{debug, error, info, warn};

use crate::AppState;
use crate::models::ChannelId;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

/// Handler for the WebSocket upgrade request.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(channel_id_str): Path<String>,
    State(state): State<AppState>,
) -> Response {
    // Parse the string channel_id into a Uuid
    let channel_id = match uuid::Uuid::parse_str(&channel_id_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            warn!("Invalid Channel ID format received: {}", channel_id_str);
            return (StatusCode::BAD_REQUEST, "Invalid Channel ID format").into_response();
        }
    };

    ws.max_message_size(state.config.max_message_size)
        .on_upgrade(move |socket| handle_websocket(socket, channel_id, state))
}

/// Handles the established WebSocket connection for a participant in a channel.
pub async fn handle_websocket(socket: WebSocket, channel_id: ChannelId, state: AppState) {
    info!("New connection attempt for channel: {}", channel_id);

    // Split the WebSocket into a sender and a receiver.
    let (mut sender_ws, mut receiver_ws) = socket.split();

    // Create a Multi-Producer, Single-Consumer (mpsc) channel for this specific connection.
    let (tx, mut rx) = mpsc::channel::<Message>(100); // Buffer size of 100 messages
    let mut tx_clone = tx.clone();

    // Try to add this participant's sender to the channel manager.
    let participant_id_opt = state
        .channel_manager
        .add_participant_to_channel(channel_id, tx, &state.config.max_channel_count)
        .await;

    let participant_id = if let Some(id) = participant_id_opt {
        info!("Participant {} joined channel: {}", id, channel_id);
        id
    } else {
        warn!(
            "Channel {} is full or max channel limit reached. Rejecting new connection.",
            channel_id
        );
        // Channel is full or max channel limit reached, send a close message to the client immediately
        let _ = sender_ws
            .send(Message::Close(Some(CloseFrame {
                code: 1008, // Policy Violation
                reason: "Channel is full or server max channel limit reached".into(),
            })))
            .await;
        let _ = sender_ws.close().await; // Ensure the socket is closed
        return; // Exit handler
    };

    // Spawn a task to forward messages from our mpsc channel (tx) to the WebSocket sender (sender_ws).
    // This task runs concurrently, allowing other participants to send messages to this client.
    tokio::spawn(async move {
        while let Some(msg) = rx.next().await {
            if let Err(e) = sender_ws.send(msg).await {
                error!(
                    "Failed to send message via WebSocket sender for channel {}: {}",
                    channel_id, e
                );
                break; // Break the loop if sending fails (e.g., connection closed)
            }
        }
        info!(
            "WebSocket send task for participant {} in channel {} finished.",
            participant_id, channel_id
        );
    });

    // Initialize rate limiter
    let rate_limiter = RateLimiter::direct(
        Quota::per_second(
            NonZeroU32::new(state.config.rate_limit_per_second).unwrap_or(NonZeroU32::MIN),
        )
        .allow_burst(NonZeroU32::new(state.config.rate_limit_burst).unwrap_or(NonZeroU32::MIN)),
    );

    // Loop to receive messages from the client via the WebSocket receiver.
    while let Some(msg) = receiver_ws.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if rate_limiter.check().is_err() {
                        warn!(
                            "Rate limit exceeded for participant {} in channel {}",
                            participant_id, channel_id
                        );
                        if let Err(e) = tx_clone
                            .send(Message::Text(
                                "Error: Rate limit exceeded.".to_string().into(),
                            ))
                            .await
                        {
                            error!(
                                "Failed to send rate limit warning to participant {} in channel {}: {}",
                                participant_id, channel_id, e
                            );
                        }
                        continue;
                    }

                    debug!(
                        "Received message from participant {} in channel {}: {}",
                        participant_id, channel_id, text
                    );
                    // Broadcast the received message to other participants in the channel
                    state
                        .channel_manager
                        .broadcast_message_in_channel(
                            &channel_id,
                            Message::Text(text),
                            participant_id, // The ID of the sender, so it's not sent back
                        )
                        .await;
                }
                Message::Close(frame) => {
                    info!(
                        "WebSocket connection closed for participant {} in channel {}. Code: {:?}, Reason: {:?}.",
                        participant_id,
                        channel_id,
                        frame.as_ref().map(|f| f.code),
                        frame.as_ref().map(|f| &f.reason)
                    );
                    break; // Exit the loop on close message
                }
                Message::Ping(payload) => {
                    // Respond to pings with a pong
                    if let Err(e) = tx_clone.send(Message::Pong(payload)).await {
                        error!(
                            "Failed to send pong for participant {} in channel {}: {}",
                            participant_id, channel_id, e
                        );
                    }
                }
                // Ignore other message types like Binary
                _ => {}
            }
        } else {
            // An error occurred while receiving a message (e.g., connection reset).
            error!(
                "Error receiving message for participant {} in channel {}: {:?}.",
                participant_id, channel_id, msg
            );
            break; // Exit the loop on error
        }
    }

    // Final cleanup when the message receiving loop finishes (connection closed or error).
    info!(
        "WebSocket handler for participant {} in channel {} finished. Removing participant.",
        participant_id, channel_id
    );
    state
        .channel_manager
        .remove_participant_from_channel(&channel_id, participant_id)
        .await;
}

/// Simple healthcheck route.
pub async fn healthcheck() -> &'static str {
    "OK"
}
