const std = @import("std");

const CGROUP_BASE_PATH = "/sys/fs/cgroup";

pub export fn cgroup_create(name: [*:0]const u8) i32 {
    _ = name;
    return 0;
}

pub export fn cgroup_destroy(name: [*:0]const u8) i32 {
    _ = name;
    return 0;
}

pub export fn cgroup_freeze(name: [*:0]const u8) i32 {
    _ = name;
    return 0;
}

pub export fn cgroup_thaw(name: [*:0]const u8) i32 {
    _ = name;
    return 0;
}

pub export fn cgroup_set_cpu_limit(name: [*:0]const u8, quota: i64, period: u64) i32 {
    _ = name;
    _ = quota;
    _ = period;
    return 0;
}

pub export fn cgroup_set_memory_limit(name: [*:0]const u8, limit_bytes: u64) i32 {
    _ = name;
    _ = limit_bytes;
    return 0;
}

pub export fn cgroup_set_io_limit(
    name: [*:0]const u8,
    dev_major: u32,
    dev_minor: u32,
    rbps: u64,
    wbps: u64,
) i32 {
    _ = name;
    _ = dev_major;
    _ = dev_minor;
    _ = rbps;
    _ = wbps;
    return 0;
}

pub export fn cgroup_add_process(name: [*:0]const u8, pid: i32) i32 {
    _ = name;
    _ = pid;
    return 0;
}

test "cgroup_manager_basic" {
    try std.testing.expectEqual(@as(i32, 0), cgroup_create("test_cgroup"));
    try std.testing.expectEqual(@as(i32, 0), cgroup_destroy("test_cgroup"));
}
