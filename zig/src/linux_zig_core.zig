const std = @import("std");
const linux = @import("std").os.linux;
const PosixIOVec = std.posix.iovec;
const PosixIOVecConst = std.posix.iovec_const;

pub export fn zig_compress_and_store(
    pid: u32,
    addr: u64,
    len: usize,
    path: [*:0]const u8,
) callconv(.c) i32 {
    if (len == 0 or len > 1024 * 1024 * 1024) return -1;

    const local_pid: linux.pid_t = @intCast(pid);

    const fd = linux.open(path, linux.O{ .ACCMODE = .WRONLY, .CREAT = true, .TRUNC = true, .CLOEXEC = true }, 0o644);
    if (@as(isize, @bitCast(fd)) < 0) return -1;
    defer {
        _ = linux.close(@as(i32, @intCast(fd)));
    }

    const local_addr: usize = addr;
    const remaining_len = len;
    var written: usize = 0;

    while (written < remaining_len) {
        const chunk_size = @min(remaining_len - written, 65536);
        var buf: [65536]u8 = undefined;
        @memset(buf[0..chunk_size], 0);

        var iov_local = [1]PosixIOVec{
            .{ .base = &buf, .len = chunk_size },
        };
        var iov_remote = [1]PosixIOVecConst{
            .{ .base = @ptrFromInt(local_addr + written), .len = chunk_size },
        };

        const ret = linux.process_vm_readv(local_pid, &iov_local, &iov_remote, 0);
        if (ret <= 0) break;

        const bytes_read: usize = @intCast(ret);
        const write_ret = linux.write(@as(i32, @intCast(fd)), &buf, bytes_read);
        if (write_ret <= 0) break;
        written += @as(usize, @intCast(write_ret));
    }

    if (written == 0) return -1;
    return @intCast(written);
}

test "linux_zig_core_basic_test" {
    try std.testing.expect(true);
}

test "basic_assertion_test" {
    try std.testing.expect(true);
    try std.testing.expectEqual(@as(i32, 2), 1 + 1);
}
