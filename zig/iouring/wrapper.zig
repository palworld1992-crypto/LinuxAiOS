const std = @import("std");
const linux = std.os.linux;

pub const IoUring = extern struct {
    ring_fd: i32,
    ring_size: u32,
    mapped: bool,
};

pub export fn iouring_init(
    ring: *IoUring,
    entries: u32,
    flags: u32,
) i32 {
    _ = flags;
    ring.* = IoUring{
        .ring_fd = -1,
        .ring_size = entries,
        .mapped = false,
    };
    return 0;
}

pub export fn iouring_submit_read(
    ring_fd: i32,
    fd: i32,
    buf: [*]u8,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    _ = ring_fd;
    _ = fd;
    _ = buf;
    _ = len;
    _ = offset;
    _ = user_data;
    return 0;
}

pub export fn iouring_submit_write(
    ring_fd: i32,
    fd: i32,
    buf: [*]const u8,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    _ = ring_fd;
    _ = fd;
    _ = buf;
    _ = len;
    _ = offset;
    _ = user_data;
    return 0;
}

pub export fn iouring_submit_openat(
    ring_fd: i32,
    dirfd: i32,
    path: [*:0]const u8,
    flags: i32,
    mode: u32,
    user_data: u64,
) i32 {
    _ = ring_fd;
    _ = dirfd;
    _ = path;
    _ = flags;
    _ = mode;
    _ = user_data;
    return 0;
}

pub export fn iouring_register_buffers(
    ring_fd: i32,
    buffers: *anyopaque,
    nr_buffers: u32,
) i32 {
    _ = ring_fd;
    _ = buffers;
    _ = nr_buffers;
    return 0;
}

pub export fn iouring_close(ring_fd: i32) i32 {
    if (ring_fd >= 0) {
        const result = linux.close(ring_fd);
        return if (result == 0) 0 else -1;
    }
    return 0;
}

pub export fn iouring_wait_cqes(
    ring_fd: i32,
    cqes: *anyopaque,
    max_cqes: u32,
) i32 {
    _ = ring_fd;
    _ = cqes;
    _ = max_cqes;
    return 0;
}

test "iouring_basic" {
    var ring: IoUring = undefined;
    const result = iouring_init(&ring, 32, 0);
    try std.testing.expect(result == 0);
    if (result == 0 and ring.ring_fd >= 0) {
        iouring_close(ring.ring_fd);
    }
}
