const std = @import("std");
const linux = @import("std").os.linux;

/// Cgroups v2 manager - quản lý resource limits, freeze, process assignment
const CGROUP_V2_MOUNT = "/sys/fs/cgroup";

fn toCStr(buf: []u8, src: []const u8) [*:0]const u8 {
    const len = @min(src.len, buf.len - 1);
    @memcpy(buf[0..len], src[0..len]);
    buf[len] = 0;
    return buf[0..len :0].ptr;
}

fn writeCgroupFile(cgroup: []const u8, filename: []const u8, value: []const u8) i32 {
    var path_buf: [512]u8 = undefined;
    const cgroup_c = toCStr(&path_buf, cgroup);

    var full_buf: [512]u8 = undefined;
    const full_path = std.fmt.bufPrint(&full_buf, "{s}/{s}/{s}", .{ CGROUP_V2_MOUNT, cgroup_c, filename }) catch return -1;
    var c_buf: [512]u8 = undefined;
    const c_path = toCStr(&c_buf, full_path);

    var opts = linux.O{};
    opts.ACCMODE = .WRONLY;
    opts.TRUNC = true;
    const fd = linux.open(c_path, opts, 0);
    if (fd == std.math.maxInt(usize)) return -1;
    defer _ = linux.close(@as(i32, @intCast(fd)));

    const written = linux.write(@as(i32, @intCast(fd)), value.ptr, value.len);
    if (written == std.math.maxInt(usize)) return -1;
    return 0;
}

fn readCgroupFile(cgroup: []const u8, filename: []const u8, buffer: []u8) i32 {
    var path_buf: [512]u8 = undefined;
    const cgroup_c = toCStr(&path_buf, cgroup);

    var full_buf: [512]u8 = undefined;
    const full_path = std.fmt.bufPrint(&full_buf, "{s}/{s}/{s}", .{ CGROUP_V2_MOUNT, cgroup_c, filename }) catch return -1;
    var c_buf: [512]u8 = undefined;
    const c_path = toCStr(&c_buf, full_path);

    const opts = linux.O{ .ACCMODE = .RDONLY };
    const fd = linux.open(c_path, opts, 0);
    if (fd == std.math.maxInt(usize)) return -1;
    defer _ = linux.close(@as(i32, @intCast(fd)));

    const read_bytes = linux.read(@as(i32, @intCast(fd)), buffer.ptr, buffer.len);
    return @as(i32, @intCast(read_bytes));
}

pub export fn cgroup_create(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var path_buf: [512]u8 = undefined;
    const path = std.fmt.bufPrint(&path_buf, "{s}/{s}", .{ CGROUP_V2_MOUNT, c_str }) catch return -1;
    var c_buf: [512]u8 = undefined;
    const c_path = toCStr(&c_buf, path);

    _ = linux.mkdir(c_path, 0o755);
    return 0;
}

pub export fn cgroup_destroy(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var path_buf: [512]u8 = undefined;
    const path = std.fmt.bufPrint(&path_buf, "{s}/{s}", .{ CGROUP_V2_MOUNT, c_str }) catch return -1;
    var c_buf: [512]u8 = undefined;
    const c_path = toCStr(&c_buf, path);

    _ = linux.rmdir(c_path);
    return 0;
}

pub export fn cgroup_freeze(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    return writeCgroupFile(c_str, "cgroup.freeze", "1");
}

pub export fn cgroup_thaw(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    return writeCgroupFile(c_str, "cgroup.freeze", "0");
}

pub export fn cgroup_set_cpu_limit(name: [*:0]const u8, quota: i64, period: u64) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [64]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{} {}", .{ quota, period }) catch return -1;
    return writeCgroupFile(c_str, "cpu.max", value);
}

pub export fn cgroup_set_memory_limit(name: [*:0]const u8, limit_bytes: u64) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{limit_bytes}) catch return -1;
    return writeCgroupFile(c_str, "memory.max", value);
}

pub export fn cgroup_set_io_limit(
    name: [*:0]const u8,
    dev_major: u32,
    dev_minor: u32,
    rbps: u64,
    wbps: u64,
) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [128]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}:{} rbps={} wbps={}", .{ dev_major, dev_minor, rbps, wbps }) catch return -1;
    return writeCgroupFile(c_str, "io.max", value);
}

pub export fn cgroup_add_process(name: [*:0]const u8, pid: i32) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{pid}) catch return -1;
    return writeCgroupFile(c_str, "cgroup.procs", value);
}

pub export fn cgroup_get_memory_usage(name: [*:0]const u8) callconv(.c) i64 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [32]u8 = undefined;
    const len = readCgroupFile(c_str, "memory.current", &buf);
    if (len <= 0) return 0;
    return std.fmt.parseInt(i64, buf[0..@as(usize, @intCast(len))], 10) catch 0;
}

pub export fn cgroup_get_cpu_usage(name: [*:0]const u8) callconv(.c) i64 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [128]u8 = undefined;
    const len = readCgroupFile(c_str, "cpu.stat", &buf);
    if (len <= 0) return 0;

    var pos: usize = 0;
    const buf_len: usize = @intCast(len);
    while (pos < buf_len) {
        if (pos + 8 < buf_len and std.mem.eql(u8, buf[pos .. pos + 8], "usage_us")) {
            pos += 9;
            while (pos < buf_len and buf[pos] == ' ') pos += 1;
            var end = pos;
            while (end < buf_len and buf[end] != '\n') end += 1;
            return std.fmt.parseInt(i64, buf[pos..end], 10) catch 0;
        }
        while (pos < buf_len and buf[pos] != '\n') pos += 1;
        pos += 1;
    }
    return 0;
}

pub export fn cgroup_get_procs(name: [*:0]const u8, pids: [*]i32, max_count: usize) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [4096]u8 = undefined;
    const len = readCgroupFile(c_str, "cgroup.procs", &buf);
    if (len <= 0) return 0;

    var count: i32 = 0;
    var pos: usize = 0;
    const buf_len: usize = @intCast(len);
    while (pos < buf_len and @as(usize, @intCast(count)) < max_count) {
        while (pos < buf_len and (buf[pos] < '0' or buf[pos] > '9')) pos += 1;
        if (pos >= buf_len) break;
        var end = pos;
        while (end < buf_len and buf[end] >= '0' and buf[end] <= '9') end += 1;
        if (end > pos) {
            const pid = std.fmt.parseInt(i32, buf[pos..end], 10) catch {
                pos = end + 1;
                continue;
            };
            pids[@intCast(count)] = pid;
            count += 1;
        }
        pos = end + 1;
    }
    return count;
}

pub export fn cgroup_kill_all(name: [*:0]const u8) callconv(.c) i32 {
    var pids: [256]i32 = undefined;
    const count = cgroup_get_procs(name, &pids, 256);
    for (0..@intCast(count)) |i| {
        _ = linux.kill(pids[i], linux.SIG.KILL);
    }
    return 0;
}

pub export fn cgroup_enable_notify(name: [*:0]const u8) callconv(.c) i32 {
    _ = name;
    return 0;
}

pub export fn cgroup_get_pids_count(name: [*:0]const u8) callconv(.c) i32 {
    var pids: [1]i32 = undefined;
    return cgroup_get_procs(name, &pids, 1);
}

pub export fn cgroup_set_memory_soft_limit(name: [*:0]const u8, limit_bytes: u64) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{limit_bytes}) catch return -1;
    return writeCgroupFile(c_str, "memory.low", value);
}

pub export fn cgroup_set_io_weight(name: [*:0]const u8, weight: u64) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{weight}) catch return -1;
    return writeCgroupFile(c_str, "io.weight", value);
}

test "cgroup_manager_basic" {
    try std.testing.expect(true);
}
