use scc::ConnectionManager;
use tokio::sync::mpsc;

#[test]
fn test_connection_manager_creation() {
    let _mgr = ConnectionManager::new();
}

#[test]
fn test_connection_manager_default() {
    let _mgr = ConnectionManager::default();
}

#[test]
fn test_register_peer_and_send() -> Result<(), Box<dyn std::error::Error>> {
    let mgr = ConnectionManager::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    mgr.register_peer("peer_1".to_string(), tx);

    let data = vec![1, 2, 3, 4];
    mgr.send("peer_1", data.clone())?;

    let received = rx.try_recv()?;
    assert_eq!(received, data);
    Ok(())
}

#[test]
fn test_send_to_unknown_peer() {
    let mgr = ConnectionManager::new();
    let result = mgr.send("unknown_peer", vec![1, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn test_broadcast() -> Result<(), Box<dyn std::error::Error>> {
    let mgr = ConnectionManager::new();

    let (tx1, mut rx1) = mpsc::unbounded_channel::<Vec<u8>>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<Vec<u8>>();
    let (tx3, mut rx3) = mpsc::unbounded_channel::<Vec<u8>>();

    mgr.register_peer("peer_1".to_string(), tx1);
    mgr.register_peer("peer_2".to_string(), tx2);
    mgr.register_peer("peer_3".to_string(), tx3);

    let data = vec![0xAA, 0xBB, 0xCC];
    mgr.broadcast(data.clone());

    assert_eq!(rx1.try_recv()?, data);
    assert_eq!(rx2.try_recv()?, data);
    assert_eq!(rx3.try_recv()?, data);
    Ok(())
}

#[test]
fn test_broadcast_empty_peers() {
    let mgr = ConnectionManager::new();
    mgr.broadcast(vec![1, 2, 3]);
}

#[test]
fn test_multiple_peers() -> Result<(), Box<dyn std::error::Error>> {
    let mgr = ConnectionManager::new();
    let mut receivers = vec![];

    for i in 0..10 {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
        mgr.register_peer(format!("peer_{}", i), tx);
        receivers.push(rx);
    }

    let data = vec![0xFF];
    for i in 0..10 {
        mgr.send(&format!("peer_{}", i), data.clone())?;
    }

    for rx in &mut receivers {
        let received = rx.try_recv()?;
        assert_eq!(received, data);
    }
    Ok(())
}
