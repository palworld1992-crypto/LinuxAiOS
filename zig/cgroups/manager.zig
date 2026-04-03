const std = @import("std");
const linux = std.os.linux;

const CGROUP_BASE_PATH = "/sys/fs/cgroup";

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR: i32 = 2;
const O_CREAT: i32 = 64;
const O_TRUNC: i32 = 512;
const O_DIRECTORY: i32 = 0x10000;

const S_ISDIR: u32 = 0o40000;
const PATH_MAX: usize = 4096;

/// Convert a c-string to a null-terminated string slice
fn get_cgroup_path(name: [*]const u8) [:0]u8 {
    const name_str = std.mem.sliceTo(name, 0);
    var path: [PATH_MAX]u8 = undefined;
    const full_path = std.fmt.bufPrint(&path, "{s}/{s}", .{ CGROUP_BASE_PATH, name_str }) catch unreachable;
    return @as([:0]u8, @constCast(full_path))[0..full_path.len :0].*;
}

/// Check if a path is a directory
fn is_directory(path: [:0]const u8) !bool {
    var attr: linux.struct_stat = undefined;
    const rc = linux.stat(path.ptr, &attr);
    if (rc != 0) return linux.getErrno(rc) == @intFromEnum(linux.E.ISDIR);

    return S_ISDIR(attr.st_mode);
}

/// Create a directory at the given path
fn create_directory(path: [:0]const u8) !i32 {
    if ((try is_directory(path)) or (linux.mkdir(path.ptr, 0o755) != -1))
        return 0;

    const dir = linux.syscall3(160, @intFromPtr(path.ptr), O_CREAT | O_RDWR | O_DIRECTORY, 0o755);
    return @intCast(dir);
}

/// Write the given content to a file at the specified path
fn write_to_file(path: [:0]const u8, content: [:0]const u8) !i32 {
    const fd = linux.open(path.ptr, O_WRONLY | O_TRUNC, 0);
    if (fd < 0) return fd;
    defer _ = linux.close(@intCast(fd));

    const written = linux.write(@intCast(fd), content.ptr, content.len);
    return @intCast(written);
}

/// Read up to `buf.len` bytes from a file at the specified path
fn read_from_file(path: [:0]const u8, buf: []u8) !i32 {
    const fd = linux.open(path.ptr, O_RDONLY, 0);
    if (fd < 0) return fd;
    defer _ = linux.close(@intCast(fd));

    return linux.read(@intCast(fd), buf.ptr, buf.len);
}

/// Check if the system is running cgroup v2
fn is_cgroup_v2() bool {
    var path: [64]u8 = undefined;
    const cgroup_path = std.fmt.bufPrintZ(&path, "/sys/fs/cgroup/cgroup.controllers") catch return false;
    var attr: linux.struct_stat = undefined;
    return linux.stat(cgroup_path.ptr, &attr) == 0;
}

/// Create a cgroup with the given name
pub export fn cgroup_create(name: [*]const u8) i32 {
    const path = get_cgroup_path(name);
    const dir = create_directory(path) catch return -1;

    if (dir < 0 and linux.getErrno(dir) != @intFromEnum(linux.E.EXIST)) return dir;
    return 0;
}

/// Destroy a cgroup with the given name
pub export fn cgroup_destroy(name: [*]const u8) i32 {
    const path = get_cgroup_path(name);
    if (linux.unlink(path.ptr) < 0) return -1;

    // Remove any remaining files in the directory
    var dir_iter = std.fs.cwd().openDir(.{ .path = name, .mode = .ReadOnly }) catch return -1;
    defer _ = dir_iter.close();

    while (true) {
        const entry = try dir_iter.readEntry();
        if (entry == null) break;

        const sub_path = std.fmt.allocPrint(std.heap.page_allocator, "{s}/{s}", .{ name, entry.name }) catch unreachable;
        if (std.fs.cwd().removeDirTree(sub_path)) return -1;
    }

    return 0;
}

/// Freeze a cgroup with the given name
pub export fn cgroup_freeze(name: [*]const u8) i32 {
    if (!is_cgroup_v2()) {
        var path: [PATH_MAX]u8 = undefined;
        const freeze_path = std.fmt.bufPrint(&path, "{s}/cgroup.freeze", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

        if (write_to_file(freeze_path, "1") < 0) return -1;
    }
    return 0;
}

/// Thaw a cgroup with the given name
pub export fn cgroup_thaw(name: [*]const u8) i32 {
    if (!is_cgroup_v2()) {
        var path: [PATH_MAX]u8 = undefined;
        const freeze_path = std.fmt.bufPrint(&path, "{s}/cgroup.freeze", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

        if (write_to_file(freeze_path, "0") < 0) return -1;
    }
    return 0;
}

/// Set the CPU limit for a cgroup with the given name
pub export fn cgroup_set_cpu_limit(name: [*]const u8, quota: i64, period: u64) i32 {
    var path: [PATH_MAX]u8 = undefined;
    const cgroup_path = std.fmt.bufPrint(&path, "{s}/{s}", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

    var quota_path: [PATH_MAX]u8 = undefined;
    const quota_file = std.fmt.bufPrintZ(&quota_path, "{s}/cpu.max", .{cgroup_path}) catch return -1;

    var content: [64]u8 = undefined;
    const quota_str = std.fmt.bufPrint(&content, "{} {}", .{ quota, period }) catch return -1;

    if (write_to_file(quota_file, quota_str) < 0) return -1;
    return 0;
}

/// Set the memory limit for a cgroup with the given name
pub export fn cgroup_set_memory_limit(name: [*]const u8, limit_bytes: u64) i32 {
    var path: [PATH_MAX]u8 = undefined;
    const cgroup_path = std.fmt.bufPrint(&path, "{s}/{s}/memory.max", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

    var content: [32]u8 = undefined;
    const limit_str = std.fmt.bufPrint(&content, "{}", .{limit_bytes}) catch return -1;

    if (write_to_file(cgroup_path, limit_str) < 0) return -1;
    return 0;
}

/// Set the IO limits for a cgroup with the given name
pub export fn cgroup_set_io_limit(
    name: [*]const u8,
    dev_major: u32,
    dev_minor: u32,
    rbps: u64,
    wbps: u64,
) i32 {
    var path: [PATH_MAX]u8 = undefined;
    const cgroup_path = std.fmt.bufPrint(&path, "{s}/{s}/io.max", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

    var content: [64]u8 = undefined;
    const io_str = std.fmt.bufPrint(&content, "{}:{} rbps={} wbps={}", .{ dev_major, dev_minor, rbps, wbps }) catch return -1;

    if (write_to_file(cgroup_path, io_str) < 0) return -1;
    return 0;
}

/// Add a process with the given PID to a cgroup with the given name
pub export fn cgroup_add_process(name: [*]const u8, pid: i32) i32 {
    var path: [PATH_MAX]u8 = undefined;
    const cgroup_path = std.fmt.bufPrint(&path, "{s}/{s}/cgroup.procs", .{ CGROUP_BASE_PATH, std.mem.sliceTo(name, 0) }) catch return -1;

    var content: [32]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&content, "{}", .{pid}) catch return -1;

    if (write_to_file(cgroup_path, pid_str) < 0) return -1;
    return 0;
}

test "cgroup_manager_basic" {
    try std.testing.expectEqual(@as(i32, 0), cgroup_create("test_cgroup"));
    try std.testing.expectEqual(@as(i32, 0), cgroup_destroy("test_cgroup"));
}
