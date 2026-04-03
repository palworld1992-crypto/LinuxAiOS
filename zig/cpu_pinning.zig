const std = @import("std");

const linux = std.os.linux;

pub export fn pin_thread_to_core(
    pid: i32,
    core_mask: u64,
) i32 {
    _ = pid;
    _ = core_mask;
    return 0;
}

pub export fn get_thread_affinity(
    pid: i32,
    core_mask: *u64,
) i32 {
    _ = pid;
    core_mask.* = 0;
    return 0;
}

pub export fn pin_current_thread(core: u32) i32 {
    _ = core;
    return 0;
}

pub export fn unpin_thread(pid: i32) i32 {
    _ = pid;
    return 0;
}

pub export fn get_cpu_count() i32 {
    return @as(i32, @intCast(std.Thread.getCpuCount() catch 1));
}

test "cpu_pinning_basic" {
    const count = get_cpu_count();
    try std.testing.expect(count > 0);
}
