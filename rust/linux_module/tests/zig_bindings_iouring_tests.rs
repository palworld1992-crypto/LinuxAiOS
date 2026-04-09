use linux_module::zig_bindings::iouring::{
    iouring_disable, iouring_enable, iouring_init_ring, iouring_peek_cqe, iouring_submit_read,
    iouring_submit_write, iouring_wait_cqes, is_iouring_available, try_load_iouring, IoUringHandle,
};

fn make_handle_mut() -> IoUringHandle {
    IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    }
}

#[test]
fn test_iouring_not_available_initially() {
    assert!(!is_iouring_available());
}

#[test]
fn test_iouring_init_ring_failure_when_not_available() -> anyhow::Result<()> {
    let result = iouring_init_ring(256);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_iouring_handle_struct() {
    let handle = IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    };
    assert_eq!(handle.entries, 256);
    assert_eq!(handle.ring_fd, -1);
    assert!(!handle.enabled);
}

#[test]
fn test_iouring_submit_read_when_not_available() {
    let handle = IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    };
    let mut buf = vec![0u8; 64];
    let result = iouring_submit_read(&handle, 0, 0, &mut buf, 1);
    assert!(result.is_err());
}

#[test]
fn test_iouring_submit_write_when_not_available() {
    let handle = IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    };
    let buf = vec![1u8; 64];
    let result = iouring_submit_write(&handle, 0, 0, &buf, 1);
    assert!(result.is_err());
}

#[test]
fn test_iouring_enable_when_not_available() {
    let mut handle = make_handle_mut();
    let result = iouring_enable(&mut handle);
    assert!(result.is_err());
}

#[test]
fn test_iouring_disable_when_not_available() {
    let mut handle = make_handle_mut();
    let result = iouring_disable(&mut handle);
    assert!(result.is_err());
}

#[test]
fn test_iouring_peek_cqe_when_not_available() {
    let handle = IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    };
    assert!(!iouring_peek_cqe(&handle));
}

#[test]
fn test_iouring_wait_cqes_when_not_available() {
    let handle = IoUringHandle {
        ring_fd: -1,
        entries: 256,
        enabled: false,
    };
    let result = iouring_wait_cqes(&handle, 1);
    assert!(result.is_err());
}

#[test]
fn test_try_load_iouring_with_invalid_path() {
    let result = try_load_iouring("/nonexistent/path/libiouring.so");
    assert!(result.is_err());
    assert!(!is_iouring_available());
}
