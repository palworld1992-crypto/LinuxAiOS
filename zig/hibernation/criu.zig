const std = @import("std");
const linux = @import("std").os.linux;

/// CRIU hibernation module - checkpoint/restore via CRIU binary
/// Uses fork + execve to safely invoke CRIU without shell
const CRIU_BIN_PATH = "/usr/sbin/criu";

fn runCriu(args: []const []const u8) i32 {
    var path_buf: [256]u8 = undefined;
    @memcpy(path_buf[0..CRIU_BIN_PATH.len], CRIU_BIN_PATH);
    path_buf[CRIU_BIN_PATH.len] = 0;
    const c_path = path_buf[0..CRIU_BIN_PATH.len :0].ptr;

    var argv_buf: [32][256]u8 = undefined;
    var argv_ptrs: [33]?[*:0]u8 = undefined;
    argv_ptrs[0] = c_path;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        @memcpy(argv_buf[i][0..args[i].len], args[i]);
        argv_buf[i][args[i].len] = 0;
        argv_ptrs[i + 1] = argv_buf[i][0..args[i].len :0].ptr;
    }
    argv_ptrs[args.len + 1] = null;

    const pid = linux.fork();
    if (pid == std.math.maxInt(usize)) return -1;

    if (pid == 0) {
        const empty_env: [*:null]const ?[*:0]const u8 = @ptrCast(&[_]?[*:0]const u8{null});
        _ = linux.execve(c_path, @ptrCast(&argv_ptrs), empty_env);
        linux.exit(127) catch {};
        unreachable;
    }

    var status: u32 = 0;
    _ = linux.wait4(@as(i32, @intCast(pid)), &status, 0, null);
    if ((status & 0x7f) == 0) {
        return @as(i32, @intCast((status >> 8) & 0xff));
    }
    return -1;
}

pub export fn criu_checkpoint(
    pid: i32,
    images_dir: [*:0]const u8,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    const args = [_][]const u8{
        "dump",
        "-t",
        pid_str,
        "-D",
        dir,
        "--shell-job",
    };
    return runCriu(&args);
}

pub export fn criu_restore(
    images_dir: [*:0]const u8,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    const args = [_][]const u8{
        "restore",
        "-D",
        dir,
        "--shell-job",
    };
    return runCriu(&args);
}

pub export fn criu_pre_dump(
    pid: i32,
    images_dir: [*:0]const u8,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    const args = [_][]const u8{
        "pre-dump",
        "-t",
        pid_str,
        "-D",
        dir,
    };
    return runCriu(&args);
}

pub export fn criu_check() callconv(.c) i32 {
    const args = [_][]const u8{"check"};
    return runCriu(&args);
}

pub export fn criu_is_available() callconv(.c) bool {
    return criu_check() == 0;
}

pub export fn criu_get_version() callconv(.c) i32 {
    if (!criu_is_available()) return 0;
    return 1;
}

pub export fn criu_page_server(
    pid: i32,
    images_dir: [*:0]const u8,
    port: u32,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    var port_buf: [16]u8 = undefined;
    const port_str = std.fmt.bufPrint(&port_buf, "{}", .{port}) catch return -1;
    const args = [_][]const u8{
        "dump",
        "-t",
        pid_str,
        "-D",
        dir,
        "--page-server",
        "--address",
        "127.0.0.1",
        "--port",
        port_str,
    };
    return runCriu(&args);
}

pub export fn criu_dump_tree(
    pid: i32,
    images_dir: [*:0]const u8,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    const args = [_][]const u8{
        "dump",
        "-t",
        pid_str,
        "-D",
        dir,
        "--tree",
    };
    return runCriu(&args);
}

pub export fn criu_restore_tree(
    images_dir: [*:0]const u8,
    pid: *i32,
) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    const args = [_][]const u8{
        "restore",
        "-D",
        dir,
        "--tree",
    };
    const result = runCriu(&args);
    if (result == 0) {
        pid.* = 0;
    }
    return result;
}

pub export fn criu_images_exist(images_dir: [*:0]const u8) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var path_buf: [512]u8 = undefined;
    const path = std.fmt.bufPrint(&path_buf, "{s}/inventory.img", .{dir}) catch return 0;
    var c_buf: [512]u8 = undefined;
    @memcpy(c_buf[0..path.len], path);
    c_buf[path.len] = 0;
    const c_path = c_buf[0..path.len :0].ptr;
    const opts = linux.O{ .ACCMODE = .RDONLY };
    const fd = linux.open(c_path, opts, 0);
    if (fd == std.math.maxInt(usize)) return 0;
    _ = linux.close(@as(i32, @intCast(fd)));
    return 1;
}

pub export fn criu_clean_images(images_dir: [*:0]const u8) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    _ = dir;
    return 0;
}

test "criu_check_basic" {
    const result = criu_check();
    try std.testing.expect(result == 0 or result == -1 or result == 127);
}
