use super::ChannelManager;
use axum::extract::ws::Message;
use futures::{channel::mpsc, StreamExt};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_create_channel_and_join() {
    let channel_manager = ChannelManager::new();
    let channel_id = Uuid::new_v4();
    let max_channels = 10;
    let (tx, _rx) = mpsc::channel(1);

    let participant_id = channel_manager
        .add_participant_to_channel(channel_id, tx, &max_channels)
        .await;

    assert!(participant_id.is_some());
    
    let channels = channel_manager.channels.read().await;
    assert!(channels.contains_key(&channel_id));
    let channel = channels.get(&channel_id).unwrap().lock().await;
    assert_eq!(channel.participants.len(), 1);
}

#[tokio::test]
async fn test_max_participants() {
    let channel_manager = ChannelManager::new();
    let channel_id = Uuid::new_v4();
    let max_channels = 10;
    
    // P1
    let (tx1, _rx1) = mpsc::channel(1);
    let p1 = channel_manager
        .add_participant_to_channel(channel_id, tx1, &max_channels)
        .await;
    assert!(p1.is_some());

    // P2
    let (tx2, _rx2) = mpsc::channel(1);
    let p2 = channel_manager
        .add_participant_to_channel(channel_id, tx2, &max_channels)
        .await;
    assert!(p2.is_some());

    // P3 (should fail)
    let (tx3, _rx3) = mpsc::channel(1);
    let p3 = channel_manager
        .add_participant_to_channel(channel_id, tx3, &max_channels)
        .await;
    assert!(p3.is_none());
}

#[tokio::test]
async fn test_remove_participant_and_cleanup() {
    let channel_manager = ChannelManager::new();
    let channel_id = Uuid::new_v4();
    let max_channels = 10;
    
    let (tx, _rx) = mpsc::channel(1);
    let pid = channel_manager
        .add_participant_to_channel(channel_id, tx, &max_channels)
        .await
        .unwrap();

    channel_manager.remove_participant_from_channel(&channel_id, pid).await;

    let channels = channel_manager.channels.read().await;
    assert!(!channels.contains_key(&channel_id));
}

#[tokio::test]
async fn test_broadcast_locking_isolation() {
    // This test ensures that broadcasting in one channel doesn't block another.
    // We simulate a slow receiver in Channel A and ensure Channel B can still function.
    let channel_manager = ChannelManager::new();
    let channel_a = Uuid::new_v4();
    let channel_b = Uuid::new_v4();
    let max_channels = 10;

    // Setup Channel A with a "slow" receiver (filled buffer)
    let (tx_a1, mut rx_a1) = mpsc::channel(0); // 0 buffer size means it blocks if not read
    let pid_a1 = channel_manager
        .add_participant_to_channel(channel_a, tx_a1, &max_channels)
        .await
        .unwrap();
    
    // Add sender for A
    let (tx_a2, _rx_a2) = mpsc::channel(1);
    let pid_a2 = channel_manager
        .add_participant_to_channel(channel_a, tx_a2, &max_channels)
        .await
        .unwrap();


    // Setup Channel B (normal)
    let (tx_b1, _rx_b1) = mpsc::channel(10);
    let pid_b1 = channel_manager
        .add_participant_to_channel(channel_b, tx_b1, &max_channels)
        .await
        .unwrap();

    // Spawn a task to block on broadcasting to Channel A
    let cm_clone = channel_manager.clone();
    let task_a = tokio::spawn(async move {
        cm_clone.broadcast_message_in_channel(&channel_a, Message::Text("Block me".into()), pid_a2).await;
    });

    // Give it a moment to lock Channel A
    sleep(Duration::from_millis(50)).await;

    // Now try to add a participant to Channel B. 
    // If the global lock was held by task_a, this would block indefinitely (or until timeout).
    let (tx_b2, _rx_b2) = mpsc::channel(10);
    
    // This should succeed immediately because we only hold a read lock on the map during broadcast
    // and we are operating on a different channel mutex.
    let result = tokio::time::timeout(Duration::from_millis(200), async {
            channel_manager
            .add_participant_to_channel(channel_b, tx_b2, &max_channels)
            .await
    }).await;

    assert!(result.is_ok(), "Adding participant to Channel B timed out! Global lock might be held.");
    assert!(result.unwrap().is_some());

    // Cleanup: Read from rx_a1 to unblock task_a
    let _ = rx_a1.next().await;
    let _ = task_a.await;
}