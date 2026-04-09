const std = @import("std");
const linux = std.os.linux;
const types = @import("liblinux_zig_types.zig");

fn createCpuSet(mask: u64) [16]usize {
    var set: [16]usize = undefined;
    @memset(set[0..], 0);
    var i: usize = 0;
    while (i < 64) : (i += 1) {
        const idx = @as(u6, @truncate(i));
        if ((mask >> idx) & 1 == 1) {
            const bit_idx = @as(u6, @truncate(i % 64));
            set[i / 64] |= @as(usize, 1) << bit_idx;
        }
    }
    return set;
}

pub export fn liblinux_zig_pin_thread_to_core(pid: i32, core_mask: u64) callconv(.c) i32 {
    const set = createCpuSet(core_mask);
    const target: linux.pid_t = if (pid <= 0) @as(linux.pid_t, 0) else @as(linux.pid_t, pid);
    linux.sched_setaffinity(target, set[0..]) catch return -1;
    return 0;
}

pub export fn liblinux_zig_pin_current_thread(core: u32) callconv(.c) i32 {
    const cpu_count = std.Thread.getCpuCount() catch 1;
    if (core >= @as(u32, @intCast(cpu_count))) return -1;
    const mask: u64 = @as(u64, 1) << @as(u6, @intCast(core));
    const set = createCpuSet(mask);
    linux.sched_setaffinity(0, set[0..]) catch return -1;
    return 0;
}

pub export fn liblinux_zig_get_thread_affinity(pid: i32, core_mask: *u64) callconv(.c) i32 {
    var set: [16]usize = undefined;
    @memset(set[0..], 0);
    const target: linux.pid_t = if (pid <= 0) @as(linux.pid_t, 0) else @as(linux.pid_t, pid);
    const size = linux.sched_getaffinity(target, set.len * @sizeOf(usize), &set);
    core_mask.* = 0;
    for (0..size / @sizeOf(usize)) |i| {
        core_mask.* |= set[i];
    }
    return 0;
}

pub export fn liblinux_zig_unpin_thread(pid: i32) callconv(.c) i32 {
    const cpu_count = std.Thread.getCpuCount() catch 1;
    var mask: u64 = 0;
    var i: u32 = 0;
    while (i < @as(u32, @intCast(cpu_count))) : (i += 1) {
        mask |= @as(u64, 1) << @as(u6, @intCast(i));
    }
    const set = createCpuSet(mask);
    const target: linux.pid_t = if (pid <= 0) @as(linux.pid_t, 0) else @as(linux.pid_t, pid);
    linux.sched_setaffinity(target, set[0..]) catch return -1;
    return 0;
}

pub export fn liblinux_zig_get_cpu_count() callconv(.c) i32 {
    return @intCast(std.Thread.getCpuCount() catch 1);
}
