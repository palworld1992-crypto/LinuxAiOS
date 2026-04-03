//! Tests for cgroup freeze/thaw (requires root)
use linux_module::zig_bindings;
use std::env;
use std::ffi::CString;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

#[test]
fn test_cgroup_freeze_thaw() {
    if !is_root() {
        eprintln!("Skipping cgroup test: not root");
        return;
    }

    with_temp_base(|| {
        // Tạo cgroup test trong /sys/fs/cgroup (cần root)
        let cgroup_name = "aios_test_cgroup";
        let cgroup_path = format!("/sys/fs/cgroup/{}", cgroup_name);
        fs::create_dir_all(&cgroup_path).unwrap();

        // Tạo một process con để đặt vào cgroup
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();

        // Ghi PID vào cgroup.procs
        fs::write(format!("{}/cgroup.procs", cgroup_path), pid.to_string()).unwrap();

        // Freeze cgroup
        let path_cstr = CString::new(cgroup_path.clone()).unwrap();
        let freeze_result = unsafe { zig_bindings::zig_cgroup_freeze(path_cstr.as_ptr()) };
        assert_eq!(freeze_result, 0, "Freeze should succeed");

        // Kiểm tra trạng thái freeze (đọc cgroup.freeze)
        let state =
            fs::read_to_string(format!("{}/cgroup.freeze", cgroup_path)).unwrap_or_default();
        assert_eq!(state.trim(), "1");

        // Thaw cgroup
        let thaw_result = unsafe { zig_bindings::zig_cgroup_thaw(path_cstr.as_ptr()) };
        assert_eq!(thaw_result, 0, "Thaw should succeed");

        let state =
            fs::read_to_string(format!("{}/cgroup.freeze", cgroup_path)).unwrap_or_default();
        assert_eq!(state.trim(), "0");

        // Cleanup
        child.kill().unwrap();
        fs::remove_dir_all(&cgroup_path).unwrap();
    });
}

#[test]
fn test_cgroup_freeze_nonexistent() {
    if !is_root() {
        eprintln!("Skipping test: not root");
        return;
    }

    with_temp_base(|| {
        let fake_path = CString::new("/sys/fs/cgroup/nonexistent").unwrap();
        let result = unsafe { zig_bindings::zig_cgroup_freeze(fake_path.as_ptr()) };
        assert!(result < 0, "Freezing nonexistent cgroup should fail");
    });
}
