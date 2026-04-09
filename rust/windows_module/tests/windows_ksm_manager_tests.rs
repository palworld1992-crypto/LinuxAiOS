use windows_module::executor::{KsmError, KsmManager, KsmStats};

#[test]
fn test_ksm_manager_new() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    assert!(!manager.is_enabled());
    Ok(())
}

#[test]
fn test_ksm_manager_enable_not_available() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    if !manager.is_available() {
        let result = manager.enable();
        assert!(result.is_err());
        assert!(matches!(result, Err(KsmError::NotAvailable(_))));
    }
    Ok(())
}

#[test]
fn test_ksm_stats_default() -> anyhow::Result<()> {
    let stats = KsmStats {
        pages_sharing: 1000,
        pages_shared: 500,
        pages_volatile: 200,
        full_scans: 10,
        merge_across_nodes: 1,
    };
    assert_eq!(stats.pages_sharing, 1000);
    assert_eq!(stats.merge_across_nodes, 1);
    Ok(())
}

#[test]
fn test_ksm_manager_set_pages_to_scan() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    let result = manager.set_pages_to_scan(2048);
    if manager.is_available() && manager.is_enabled() {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
    Ok(())
}

#[test]
fn test_ksm_manager_set_sleep_millis() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    let result = manager.set_sleep_millis(100);
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(
            err_msg.contains("not writable")
                || err_msg.contains("not enabled")
                || err_msg.contains("Permission denied"),
            "Expected 'not writable' or 'not enabled' error, got: {}",
            err_msg
        );
    }
    Ok(())
}

#[test]
fn test_ksm_manager_set_merge_across_nodes() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    let result = manager.set_merge_across_nodes(false);
    if manager.is_available() && manager.is_enabled() {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
    Ok(())
}

#[test]
fn test_ksm_manager_get_merge_across_nodes() -> anyhow::Result<()> {
    let manager = KsmManager::new();
    assert!(manager.get_merge_across_nodes());
    Ok(())
}