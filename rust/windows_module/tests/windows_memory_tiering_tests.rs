use windows_module::executor::{VmMemoryStats, VmMemoryTiering, VmTieringCommand};

#[test]
fn test_vm_memory_tiering_new() -> anyhow::Result<()> {
    let tiering = VmMemoryTiering::new();
    assert!(!tiering.is_connected());
    Ok(())
}

#[test]
fn test_vm_memory_tiering_connect() -> anyhow::Result<()> {
    let tiering = VmMemoryTiering::new();
    let result = tiering.connect_to_linux_module();
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_vm_memory_stats_default() -> anyhow::Result<()> {
    let stats = VmMemoryStats {
        vm_id: "test-vm".to_string(),
        total_memory_kb: 1048576,
        used_memory_kb: 524288,
        cold_pages_count: 1000,
        migration_progress: 0.5,
    };
    assert_eq!(stats.vm_id, "test-vm");
    assert_eq!(stats.total_memory_kb, 1048576);
    Ok(())
}

#[test]
fn test_vm_tiering_command_prepare() -> anyhow::Result<()> {
    let cmd = VmTieringCommand::Prepare {
        vm_id: "vm-123".to_string(),
        cold_pages: vec![1, 2, 3, 4, 5],
        target_memory_kb: 524288,
    };
    match cmd {
        VmTieringCommand::Prepare { vm_id, .. } => {
            assert_eq!(vm_id, "vm-123");
        }
        _ => anyhow::bail!("Expected Prepare command"),
    }
    Ok(())
}

#[test]
fn test_vm_tiering_command_restore() -> anyhow::Result<()> {
    let cmd = VmTieringCommand::Restore {
        vm_id: "vm-456".to_string(),
        source_path: "/tmp/migration".to_string(),
    };
    match cmd {
        VmTieringCommand::Restore { vm_id, source_path } => {
            assert_eq!(vm_id, "vm-456");
            assert_eq!(source_path, "/tmp/migration");
        }
        _ => anyhow::bail!("Expected Restore command"),
    }
    Ok(())
}

#[test]
fn test_vm_tiering_command_cancel() -> anyhow::Result<()> {
    let cmd = VmTieringCommand::Cancel {
        vm_id: "vm-789".to_string(),
    };
    match cmd {
        VmTieringCommand::Cancel { vm_id } => {
            assert_eq!(vm_id, "vm-789");
        }
        _ => anyhow::bail!("Expected Cancel command"),
    }
    Ok(())
}