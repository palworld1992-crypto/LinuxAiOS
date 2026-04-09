const std = @import("std");
const linux = std.os.linux;

fn runCriu(args: []const []const u8) i32 {
    var path_buf: [256]u8 = undefined;
    const criu_path = "/usr/sbin/criu";
    @memcpy(path_buf[0..criu_path.len], criu_path);
    path_buf[criu_path.len] = 0;
    const c_path = path_buf[0..criu_path.len :0].ptr;

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
    if ((status & 0x7f) == 0) return @as(i32, @intCast((status >> 8) & 0xff));
    return -1;
}

pub export fn liblinux_zig_criu_checkpoint(pid: i32, images_dir: [*:0]const u8) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    return runCriu(&[_][]const u8{ "dump", "-t", pid_str, "-D", dir, "--shell-job" });
}

pub export fn liblinux_zig_criu_restore(images_dir: [*:0]const u8) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    return runCriu(&[_][]const u8{ "restore", "-D", dir, "--shell-job" });
}

pub export fn liblinux_zig_criu_pre_dump(pid: i32, images_dir: [*:0]const u8) callconv(.c) i32 {
    const dir = std.mem.sliceTo(images_dir, 0);
    var pid_buf: [16]u8 = undefined;
    const pid_str = std.fmt.bufPrint(&pid_buf, "{}", .{pid}) catch return -1;
    return runCriu(&[_][]const u8{ "pre-dump", "-t", pid_str, "-D", dir });
}

pub export fn liblinux_zig_criu_check() callconv(.c) i32 {
    return runCriu(&[_][]const u8{"check"});
}
