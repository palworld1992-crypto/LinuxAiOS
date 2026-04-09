const std = @import("std");
const linux = std.os.linux;
const types = @import("liblinux_zig_types.zig");

const IORING_OP_READ: u8 = 22;
const IORING_OP_WRITE: u8 = 23;

fn syscallResult(result: usize) i32 {
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

pub export fn liblinux_zig_iouring_init(ring: *anyopaque, entries: u32) callconv(.c) i32 {
    const r: *types.IoUring = @ptrCast(@alignCast(ring));
    var params: types.IoUringParams = undefined;
    @memset(@as([*]u8, @ptrCast(&params))[0..@sizeOf(types.IoUringParams)], 0);
    params.sq_entries = entries;
    params.cq_entries = entries * 2;
    const fd = syscallResult(linux.syscall3(.io_uring_setup, entries, @intFromPtr(&params), 0));
    if (fd < 0) {
        r.* = types.IoUring{ .ring_fd = -1, .ring_size = 0, .mapped = false };
        return -1;
    }
    r.* = types.IoUring{ .ring_fd = fd, .ring_size = entries, .mapped = false };
    return 0;
}

fn submit_sqe(
    ring_fd: i32,
    opcode: u8,
    fd: i32,
    addr: u64,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    if (ring_fd < 0) return -1;

    var sqe: types.IoUringSqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(types.IoUringSqe)], 0);
    sqe.opcode = opcode;
    sqe.fd = fd;
    sqe.addr = addr;
    sqe.len = @intCast(len);
    sqe.off = offset;
    sqe.user_data = user_data;

    const ret = linux.write(ring_fd, @as(*const anyopaque, @ptrCast(&sqe)), @sizeOf(types.IoUringSqe));
    if (ret < 0) return -1;

    const submitted = syscallResult(linux.syscall6(.io_uring_enter, @as(u64, @intCast(ring_fd)), 1, 0, 0, 0, 0));
    return submitted;
}

pub export fn liblinux_zig_iouring_submit_read(ring_fd: i32, fd: i32, buf: [*]u8, len: usize, offset: u64, user_data: u64) callconv(.c) i32 {
    if (ring_fd < 0) {
        const result = linux.pread(fd, buf[0..len], @intCast(offset));
        if (result < 0) return -1;
        return 0;
    }
    return submit_sqe(ring_fd, IORING_OP_READ, fd, @intFromPtr(buf), len, offset, user_data);
}

pub export fn liblinux_zig_iouring_submit_write(ring_fd: i32, fd: i32, buf: [*]const u8, len: usize, offset: u64, user_data: u64) callconv(.c) i32 {
    if (ring_fd < 0) {
        const result = linux.pwrite(fd, buf[0..len], @intCast(offset));
        if (result < 0) return -1;
        return 0;
    }
    return submit_sqe(ring_fd, IORING_OP_WRITE, fd, @intFromPtr(buf), len, offset, user_data);
}

pub export fn liblinux_zig_iouring_register_buffers(ring_fd: i32, buffers: *anyopaque, nr_buffers: u32) callconv(.c) i32 {
    const result = linux.syscall4(.io_uring_register, @as(u64, @intCast(ring_fd)), 0, @intFromPtr(buffers), nr_buffers);
    return syscallResult(result);
}

pub export fn liblinux_zig_iouring_close(ring_fd: i32) callconv(.c) i32 {
    _ = linux.close(ring_fd);
    return 0;
}

pub export fn liblinux_zig_iouring_wait_cqes(ring_fd: i32, max_cqes: u32) callconv(.c) i32 {
    if (ring_fd < 0) return -1;
    return syscallResult(linux.syscall6(.io_uring_enter, @as(u64, @intCast(ring_fd)), 0, max_cqes, 0, 0, 0));
}

pub export fn liblinux_zig_iouring_submit_and_wait(ring_fd: i32, to_submit: u32, min_complete: u32) callconv(.c) i32 {
    if (ring_fd < 0) return -1;
    return syscallResult(linux.syscall6(.io_uring_enter, @as(u64, @intCast(ring_fd)), to_submit, min_complete, 0, 0, 0));
}
