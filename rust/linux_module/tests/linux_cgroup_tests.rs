//! Tests for cgroup freeze/thaw (requires root)
use linux_module::zig_bindings::{freeze_cgroup, thaw_cgroup};
use std::env;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

fn is_root() -> bool {
    // SAFETY: getuid() is a simple read-only syscall that returns the real user ID.
    // It's always safe to call and cannot cause undefined behavior.
    unsafe { libc::getuid() == 0 }
}

#[test]
fn test_cgroup_freeze_thaw() -> anyhow::Result<()> {
    if !is_root() {
        tracing::info!("Skipping cgroup test: not root");
        return Ok(());
    }

    with_temp_base(|| {
        let cgroup_name = "aios_test_cgroup";
        let cgroup_path = format!("/sys/fs/cgroup/{}", cgroup_name);
        fs::create_dir_all(&cgroup_path)?;

        let mut child = std::process::Command::new("sleep").arg("30").spawn()?;
        let pid = child.id();

        fs::write(format!("{}/cgroup.procs", cgroup_path), pid.to_string())?;

        let freeze_result = freeze_cgroup(&cgroup_path);
        assert!(
            freeze_result.is_ok(),
            "Freeze should succeed: {:?}",
            freeze_result
        );

        let state = fs::read_to_string(format!("{}/cgroup.freeze", cgroup_path))?;
        assert_eq!(state.trim(), "1");

        let thaw_result = thaw_cgroup(&cgroup_path);
        assert!(
            thaw_result.is_ok(),
            "Thaw should succeed: {:?}",
            thaw_result
        );

        let state = fs::read_to_string(format!("{}/cgroup.freeze", cgroup_path))?;
        assert_eq!(state.trim(), "0");

        let _ = child.kill();
        let _ = fs::remove_dir_all(&cgroup_path);
        Ok(())
    })
}

#[test]
fn test_cgroup_freeze_nonexistent() -> anyhow::Result<()> {
    if !is_root() {
        tracing::info!("Skipping test: not root");
        return Ok(());
    }

    with_temp_base(|| {
        let result = freeze_cgroup("/sys/fs/cgroup/nonexistent");
        assert!(result.is_err(), "Freezing nonexistent cgroup should fail");
        Ok(())
    })
}

#[test]
fn test_cgroup_thaw_nonexistent() -> anyhow::Result<()> {
    if !is_root() {
        tracing::info!("Skipping test: not root");
        return Ok(());
    }

    with_temp_base(|| {
        let result = thaw_cgroup("/sys/fs/cgroup/nonexistent");
        assert!(result.is_err(), "Thawing nonexistent cgroup should fail");
        Ok(())
    })
}
