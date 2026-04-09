const std = @import("std");
const posix = std.posix;

pub fn io_uring_submit_read(fd: i32, buf: [*]u8, len: usize, offset: u64) i32 {
    const slice = buf[0..len];
    const ret = posix.pread(fd, slice, @intCast(offset)) catch return -1;
    return @intCast(ret);
}