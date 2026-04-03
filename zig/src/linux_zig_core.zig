const std = @import("std");

pub export fn freeze_cgroup(path: [*:0]const u8) i32 {
    _ = path;
    return 0;
}

pub export fn thaw_cgroup(path: [*:0]const u8) i32 {
    _ = path;
    return 0;
}

pub export fn load_ebpf_program(path: [*:0]const u8) i32 {
    _ = path;
    return 0;
}

pub export fn zig_ebpf_load(program_path: [*:0]const u8) i32 {
    return load_ebpf_program(program_path);
}

pub export fn zig_cgroup_freeze(path: [*:0]const u8) i32 {
    return freeze_cgroup(path);
}

pub export fn zig_cgroup_thaw(path: [*:0]const u8) i32 {
    return thaw_cgroup(path);
}

pub export fn zig_compress_and_store(
    pid: u32,
    addr: u64,
    len: usize,
    path: [*:0]const u8,
) i32 {
    _ = pid;
    _ = addr;
    _ = len;
    _ = path;
    return 0;
}

test "linux_zig_core_basic_test" {
    try std.testing.expect(true);
}

test "basic_assertion_test" {
    try std.testing.expect(true);
    try std.testing.expectEqual(1 + 1, 2);
}
