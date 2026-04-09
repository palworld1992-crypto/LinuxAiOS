const std = @import("std");
const linux = std.os.linux;

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
    var opts = std.os.linux.O{};
    opts.ACCMODE = .WRONLY;
    opts.TRUNC = true;
    const fd = std.os.linux.open(c_path, opts, 0);
    if (fd == std.math.maxInt(usize)) return -1;
    defer _ = std.os.linux.close(@as(i32, @intCast(fd)));
    const written = std.os.linux.write(@as(i32, @intCast(fd)), value.ptr, value.len);
    if (written == std.math.maxInt(usize)) return -1;
    return 0;
}

pub export fn liblinux_zig_cgroup_create(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var path_buf: [512]u8 = undefined;
    const path = std.fmt.bufPrint(&path_buf, "{s}/{s}", .{ CGROUP_V2_MOUNT, c_str }) catch return -1;
    var c_buf: [512]u8 = undefined;
    _ = std.os.linux.mkdir(toCStr(&c_buf, path), 0o755);
    return 0;
}

pub export fn liblinux_zig_cgroup_destroy(name: [*:0]const u8) callconv(.c) i32 {
    const c_str = std.mem.sliceTo(name, 0);
    var path_buf: [512]u8 = undefined;
    const path = std.fmt.bufPrint(&path_buf, "{s}/{s}", .{ CGROUP_V2_MOUNT, c_str }) catch return -1;
    var c_buf: [512]u8 = undefined;
    _ = std.os.linux.rmdir(toCStr(&c_buf, path));
    return 0;
}

pub export fn liblinux_zig_cgroup_freeze(name: [*:0]const u8) callconv(.c) i32 {
    return writeCgroupFile(std.mem.sliceTo(name, 0), "cgroup.freeze", "1");
}

pub export fn liblinux_zig_cgroup_thaw(name: [*:0]const u8) callconv(.c) i32 {
    return writeCgroupFile(std.mem.sliceTo(name, 0), "cgroup.freeze", "0");
}

pub export fn liblinux_zig_cgroup_set_cpu_limit(name: [*:0]const u8, quota: i64, period: u64) callconv(.c) i32 {
    var buf: [64]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{} {}", .{ quota, period }) catch return -1;
    return writeCgroupFile(std.mem.sliceTo(name, 0), "cpu.max", value);
}

pub export fn liblinux_zig_cgroup_set_memory_limit(name: [*:0]const u8, limit_bytes: u64) callconv(.c) i32 {
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{limit_bytes}) catch return -1;
    return writeCgroupFile(std.mem.sliceTo(name, 0), "memory.max", value);
}

pub export fn liblinux_zig_cgroup_set_io_limit(name: [*:0]const u8, dev_major: u32, dev_minor: u32, rbps: u64, wbps: u64) callconv(.c) i32 {
    var buf: [128]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}:{} rbps={} wbps={}", .{ dev_major, dev_minor, rbps, wbps }) catch return -1;
    return writeCgroupFile(std.mem.sliceTo(name, 0), "io.max", value);
}

pub export fn liblinux_zig_cgroup_add_process(name: [*:0]const u8, pid: i32) callconv(.c) i32 {
    var buf: [32]u8 = undefined;
    const value = std.fmt.bufPrint(&buf, "{}", .{pid}) catch return -1;
    return writeCgroupFile(std.mem.sliceTo(name, 0), "cgroup.procs", value);
}
