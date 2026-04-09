const std = @import("std");
const linux = @import("std").os.linux;

/// CPU pinning module - quản lý CPU affinity cho thread/process
/// Sử dụng sched_setaffinity/sched_getaffinity syscalls
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

pub export fn pin_thread_to_core(
    pid: i32,
    core_mask: u64,
) callconv(.c) i32 {
    const set = createCpuSet(core_mask);
    const target: linux.pid_t = if (pid <= 0) @as(linux.pid_t, 0) else @as(linux.pid_t, pid);
    linux.sched_setaffinity(target, set[0..]) catch return -1;
    return 0;
}

pub export fn get_thread_affinity(
    pid: i32,
    core_mask: *u64,
) callconv(.c) i32 {
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

pub export fn pin_current_thread(core: u32) callconv(.c) i32 {
    const cpu_count = std.Thread.getCpuCount() catch 1;
    if (core >= @as(u32, @intCast(cpu_count))) {
        return -1;
    }
    const mask: u64 = @as(u64, 1) << @as(u6, @intCast(core));
    const set = createCpuSet(mask);
    linux.sched_setaffinity(0, set[0..]) catch return -1;
    return 0;
}

pub export fn unpin_thread(pid: i32) callconv(.c) i32 {
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

pub export fn get_cpu_count() callconv(.c) i32 {
    return @intCast(std.Thread.getCpuCount() catch 1);
}

pub export fn get_current_cpu() callconv(.c) i32 {
    var cpu: usize = 0;
    var node: usize = 0;
    _ = linux.getcpu(&cpu, &node);
    return @intCast(cpu);
}

pub export fn pin_thread_range(pid: i32, start_core: u32, num_cores: u32) callconv(.c) i32 {
    const cpu_count = std.Thread.getCpuCount() catch 1;
    if (start_core >= @as(u32, @intCast(cpu_count))) {
        return -1;
    }
    if (start_core + num_cores > @as(u32, @intCast(cpu_count))) {
        return -1;
    }
    var mask: u64 = 0;
    var i: u32 = start_core;
    while (i < start_core + num_cores) : (i += 1) {
        mask |= @as(u64, 1) << @as(u6, @intCast(i));
    }
    const set = createCpuSet(mask);
    const target: linux.pid_t = if (pid <= 0) @as(linux.pid_t, 0) else @as(linux.pid_t, pid);
    linux.sched_setaffinity(target, set[0..]) catch return -1;
    return 0;
}

pub export fn get_available_cores(buffer: [*]u32, max_count: usize) callconv(.c) i32 {
    const count = std.Thread.getCpuCount() catch 1;
    const n = @min(@as(usize, count), max_count);
    for (0..n) |i| {
        buffer[i] = @intCast(i);
    }
    return @intCast(n);
}

pub export fn is_core_online(core: u32) callconv(.c) bool {
    const count = std.Thread.getCpuCount() catch 1;
    return core < @as(u32, @intCast(count));
}

pub export fn get_numa_node(core: u32) callconv(.c) i32 {
    var file_path_buf: [128]u8 = undefined;
    const file_path = std.fmt.bufPrint(&file_path_buf, "/sys/devices/system/cpu/cpu{d}/node0/num_node", .{core}) catch return -1;

    const fd = linux.open(@as([*:0]const u8, @ptrCast(file_path)), linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
    if (@as(isize, @bitCast(fd)) < 0) return -1;
    defer {
        _ = linux.close(@as(i32, @intCast(fd)));
    }

    var buf: [32]u8 = undefined;
    const n = linux.read(@as(i32, @intCast(fd)), &buf, buf.len);
    if (n == 0) return -1;

    const node_str = std.mem.trim(u8, buf[0..n], &[_]u8{ '\n', '\r' });
    return std.fmt.parseInt(i32, node_str, 10) catch -1;
}

pub export fn get_core_frequency(core: u32) callconv(.c) i32 {
    var file_path_buf: [128]u8 = undefined;
    const file_path = std.fmt.bufPrint(&file_path_buf, "/sys/devices/system/cpu/cpu{d}/cpufreq/scaling_cur_freq", .{core}) catch return -1;

    const fd = linux.open(@as([*:0]const u8, @ptrCast(file_path)), linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
    if (@as(isize, @bitCast(fd)) < 0) return -1;
    defer {
        _ = linux.close(@as(i32, @intCast(fd)));
    }

    var buf: [32]u8 = undefined;
    const n = linux.read(@as(i32, @intCast(fd)), &buf, buf.len);
    if (n == 0) return -1;

    const freq_str = std.mem.trim(u8, buf[0..n], &[_]u8{ '\n', '\r' });
    return @divExact(std.fmt.parseInt(i32, freq_str, 10) catch return -1, 1000);
}

pub export fn pin_thread_to_numa_node(node: i32) callconv(.c) i32 {
    const cpu_count = std.Thread.getCpuCount() catch 1;
    var mask: u64 = 0;
    var core: u32 = 0;

    while (core < @as(u32, @intCast(cpu_count))) : (core += 1) {
        if (get_numa_node(core) == node) {
            mask |= @as(u64, 1) << @as(u6, @intCast(core));
        }
    }

    if (mask == 0) return -1;

    const set = createCpuSet(mask);
    linux.sched_setaffinity(0, set[0..]) catch return -1;
    return 0;
}

pub export fn get_cpu_usage() callconv(.c) f64 {
    const file = linux.open("/proc/stat", linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
    if (@as(isize, @bitCast(file)) < 0) return -1.0;
    defer {
        _ = linux.close(@as(i32, @intCast(file)));
    }

    var buf: [256]u8 = undefined;
    const n = linux.read(@as(i32, @intCast(file)), &buf, buf.len);
    if (n == 0) return -1.0;

    const content = buf[0..n];
    const first_line_end = std.mem.indexOf(u8, content, "\n") orelse content.len;
    const first_line = content[0..first_line_end];

    var parts = std.mem.tokenizeScalar(u8, first_line, ' ');
    _ = parts.next();

    var total: u64 = 0;
    var idle: u64 = 0;
    var iowait: u64 = 0;
    var idx: u32 = 0;

    while (parts.next()) |val_str| {
        const val = std.fmt.parseInt(u64, val_str, 10) catch break;
        total += val;
        if (idx == 3) idle = val;
        if (idx == 4) iowait = val;
        idx += 1;
    }

    if (total == 0) return 0.0;
    const idle_total = idle + iowait;
    return @as(f64, @floatFromInt(idle_total)) / @as(f64, @floatFromInt(total)) * 100.0;
}

test "cpu_pinning_basic" {
    const count = get_cpu_count();
    try std.testing.expect(count > 0);
}
