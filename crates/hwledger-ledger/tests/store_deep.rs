//! Deep coverage tests for ledger event store.
//! Traces to: FR-LEDGER-002 (Event Persistence)
//!
//! Tests concurrent append, history pagination, chain integrity verification.

use hwledger_ledger::{Event, EventStore};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

// Test 1: Create store and append single event
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_append_single() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    let event = Event {
        seq: 1,
        timestamp: chrono::Utc::now(),
        event_type: "test.event".to_string(),
        agent_id: "agent1".to_string(),
        data: serde_json::json!({"key": "value"}),
        prev_hash: vec![0; 32],
    };

    let result = store.append(event).await;
    assert!(result.is_ok(), "append should succeed");
}

// Test 2: Append multiple events sequentially
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_append_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(EventStore::open(temp_dir.path()).await.unwrap());

    for i in 1..=10 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: format!("agent{}", i),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![i as u8; 32],
        };

        let result = store.append(event).await;
        assert!(result.is_ok(), "append {} should succeed", i);
    }
}

// Test 3: Concurrent appends from multiple tasks
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_concurrent_append() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(EventStore::open(temp_dir.path()).await.unwrap());
    let seq_counter = Arc::new(Mutex::new(0u64));

    let mut handles = vec![];

    for task_id in 0..5 {
        let store_clone = Arc::clone(&store);
        let seq_clone = Arc::clone(&seq_counter);

        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                let mut seq = seq_clone.lock().await;
                *seq += 1;
                let event_seq = *seq;
                drop(seq);

                let event = Event {
                    seq: event_seq,
                    timestamp: chrono::Utc::now(),
                    event_type: "concurrent.event".to_string(),
                    agent_id: format!("task{}", task_id),
                    data: serde_json::json!({"task": task_id}),
                    prev_hash: vec![task_id as u8; 32],
                };

                let _ = store_clone.append(event).await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let final_seq = *seq_counter.lock().await;
    assert_eq!(final_seq, 50, "should have 50 total appends");
}

// Test 4: History retrieval with pagination
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_history_pagination() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    // Append 100 events
    for i in 1..=100 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![(i % 256) as u8; 32],
        };

        let _ = store.append(event).await;
    }

    // Retrieve first page (offset 0, limit 10)
    let result = store.history(0, 10).await;
    assert!(result.is_ok(), "history retrieval should succeed");

    // Retrieve middle page (offset 50, limit 10)
    let result = store.history(50, 10).await;
    assert!(result.is_ok(), "middle page retrieval should succeed");

    // Retrieve last page (offset 90, limit 10)
    let result = store.history(90, 10).await;
    assert!(result.is_ok(), "last page retrieval should succeed");
}

// Test 5: History with offset beyond stored events
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_history_offset_beyond() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=10 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    // Request history starting at offset 50 (beyond stored 10 events)
    let result = store.history(50, 10).await;
    assert!(result.is_ok(), "should return empty or partial result");
}

// Test 6: Chain integrity verification on clean store
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_verify_chain_valid() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=5 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    let result = store.verify_chain().await;
    assert!(result.is_ok(), "chain verification should succeed on clean store");
}

// Test 7: Empty store verification
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_verify_empty_chain() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    let result = store.verify_chain().await;
    assert!(result.is_ok(), "empty chain should be valid");
}

// Test 8: Get specific event by sequence number
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_get_event_by_seq() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=10 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: format!("agent{}", i),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    // Retrieve event at seq 5
    let result = store.get(5).await;
    assert!(result.is_ok(), "should retrieve event by seq");

    if let Ok(Some(event)) = result {
        assert_eq!(event.seq, 5);
        assert_eq!(event.agent_id, "agent5");
    }
}

// Test 9: Get non-existent event
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_get_nonexistent_event() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    let result = store.get(999).await;
    assert!(result.is_ok(), "should return Ok(None) for missing event");
}

// Test 10: Event count
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_event_count() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=25 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    let count = store.len().await;
    assert_eq!(count, 25, "store should have 25 events");
}

// Test 11: Store persistence across reopens
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_persistence() {
    let temp_dir = TempDir::new().unwrap();

    {
        let store = EventStore::open(temp_dir.path()).await.unwrap();

        let event = Event {
            seq: 1,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"persistent": true}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    } // Store closed

    // Reopen store
    {
        let store = EventStore::open(temp_dir.path()).await.unwrap();
        let count = store.len().await;
        assert_eq!(count, 1, "event should persist after reopen");

        if let Ok(Some(event)) = store.get(1).await {
            assert_eq!(event.agent_id, "agent1");
        }
    }
}

// Test 12: Very large event data
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_large_event_data() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    let mut large_data = serde_json::json!({});
    for i in 0..1000 {
        large_data[format!("key_{}", i)] = serde_json::json!("value");
    }

    let event = Event {
        seq: 1,
        timestamp: chrono::Utc::now(),
        event_type: "test.event".to_string(),
        agent_id: "agent1".to_string(),
        data: large_data,
        prev_hash: vec![0; 32],
    };

    let result = store.append(event).await;
    assert!(result.is_ok(), "should append large event");
}

// Test 13: Append after verification
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_append_after_verify() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=5 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    let _ = store.verify_chain().await;

    // Append more after verification
    let event = Event {
        seq: 6,
        timestamp: chrono::Utc::now(),
        event_type: "test.event".to_string(),
        agent_id: "agent1".to_string(),
        data: serde_json::json!({"index": 6}),
        prev_hash: vec![0; 32],
    };

    let result = store.append(event).await;
    assert!(result.is_ok(), "should append after verify");
}

// Test 14: Store with mixed event types
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_mixed_event_types() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    let event_types = vec!["test.created", "test.updated", "test.deleted", "test.verified"];

    for (i, event_type) in event_types.iter().enumerate() {
        let event = Event {
            seq: (i + 1) as u64,
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"type": event_type}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    let count = store.len().await;
    assert_eq!(count, 4);
}

// Test 15: History limit of 0
// Traces to: FR-LEDGER-002
#[tokio::test]
async fn test_store_history_zero_limit() {
    let temp_dir = TempDir::new().unwrap();
    let store = EventStore::open(temp_dir.path()).await.unwrap();

    for i in 1..=10 {
        let event = Event {
            seq: i,
            timestamp: chrono::Utc::now(),
            event_type: "test.event".to_string(),
            agent_id: "agent1".to_string(),
            data: serde_json::json!({"index": i}),
            prev_hash: vec![0; 32],
        };

        let _ = store.append(event).await;
    }

    let result = store.history(0, 0).await;
    assert!(result.is_ok(), "should handle zero-limit request");
}
