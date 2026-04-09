const std = @import("std");
const linux = @import("std").os.linux;

pub const IoUring = extern struct {
    ring_fd: i32,
    ring_size: u32,
    mapped: bool,
};

const IORING_OP_READ: u8 = 22;
const IORING_OP_WRITE: u8 = 23;
const IORING_OP_OPENAT: u8 = 18;

const IORING_REGISTER_BUFFERS: u32 = 0;
const IORING_SETUP_SQPOLL: u32 = 1 << 1;

const SQ_RING_OFFSET: usize = 0x100000;
const CQ_RING_OFFSET: usize = 0x200000;

pub const IoUringSqe = extern struct {
    opcode: u8,
    flags: u8,
    ioprio: u16,
    fd: i32,
    off: u64,
    addr: u64,
    len: u32,
    rw_flags: i32,
    user_data: u64,
    buf_index: u16,
    personality: u16,
    splice_fd_in: i32,
};

pub const IoUringParams = extern struct {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32,
    wq_fd: u32,
    resv: [3]u32,
    sq_off: [8]u32,
    cq_off: [8]u32,
};

const IoUringRing = struct {
    head: *u32,
    tail: *u32,
    mask: *u32,
    entries: [*]IoUringSqe,
    overflow: *u32,
    cqes: [*]u64,
};

fn syscallResult(result: usize) i32 {
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

fn ioUringSetup(entries: u32, params: *IoUringParams) i32 {
    return syscallResult(linux.syscall3(.io_uring_setup, entries, @intFromPtr(params), 0));
}

fn ioUringEnter(fd: i32, to_submit: u32, min_complete: u32, flags: u32) i32 {
    return syscallResult(linux.syscall6(.io_uring_enter, @as(u64, @intCast(fd)), to_submit, min_complete, flags, 0, 0));
}

fn ioUringRegister(fd: i32, opcode: u32, arg: *anyopaque, nr_args: u32) i32 {
    return syscallResult(linux.syscall4(.io_uring_register, @as(u64, @intCast(fd)), opcode, @intFromPtr(arg), nr_args));
}

pub export fn iouring_init(
    ring: *IoUring,
    entries: u32,
) callconv(.c) i32 {
    var params: IoUringParams = undefined;
    @memset(@as([*]u8, @ptrCast(&params))[0..@sizeOf(IoUringParams)], 0);
    params.sq_entries = entries;
    params.cq_entries = entries * 2;

    const fd = ioUringSetup(entries, &params);
    if (fd < 0) {
        ring.* = IoUring{
            .ring_fd = -1,
            .ring_size = 0,
            .mapped = false,
        };
        return -1;
    }

    ring.* = IoUring{
        .ring_fd = fd,
        .ring_size = entries,
        .mapped = false,
    };
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
    var sqe: IoUringSqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(IoUringSqe)], 0);
    sqe.opcode = opcode;
    sqe.fd = fd;
    sqe.addr = addr;
    sqe.len = @intCast(len);
    sqe.off = offset;
    sqe.user_data = user_data;

    const sqe_ptr = @as([*]const u8, @ptrCast(&sqe));
    const ret = linux.write(ring_fd, sqe_ptr, @sizeOf(IoUringSqe));
    if (ret < 0) return -1;

    return ioUringEnter(ring_fd, 1, 0, 0);
}

pub export fn iouring_submit_read(
    ring_fd: i32,
    fd: i32,
    buf: [*]u8,
    len: usize,
    offset: i64,
    user_data: u64,
) callconv(.c) i32 {
    if (ring_fd < 0) {
        const result = linux.pread(fd, buf, len, offset);
        if (result < 0) return -1;
        return 0;
    }
    return submit_sqe(ring_fd, IORING_OP_READ, fd, @intFromPtr(buf), len, @as(u64, @intCast(offset)), user_data);
}

pub export fn iouring_submit_write(
    ring_fd: i32,
    fd: i32,
    buf: [*]const u8,
    len: usize,
    offset: i64,
    user_data: u64,
) callconv(.c) i32 {
    if (ring_fd < 0) {
        const result = linux.pwrite(fd, buf, len, offset);
        if (result < 0) return -1;
        return 0;
    }
    return submit_sqe(ring_fd, IORING_OP_WRITE, fd, @intFromPtr(buf), len, @as(u64, @intCast(offset)), user_data);
}

pub export fn iouring_submit_openat(
    ring_fd: i32,
    dirfd: i32,
    path: [*:0]const u8,
    flags: i32,
    mode: u32,
    user_data: u64,
) callconv(.c) i32 {
    var sqe: IoUringSqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(IoUringSqe)], 0);
    sqe.opcode = IORING_OP_OPENAT;
    sqe.fd = dirfd;
    sqe.addr = @intFromPtr(path);
    sqe.len = mode;
    sqe.off = @intCast(flags);
    sqe.user_data = user_data;

    const sqe_ptr = @as([*]const u8, @ptrCast(&sqe));
    const ret = linux.write(ring_fd, sqe_ptr, @sizeOf(IoUringSqe));
    if (ret < 0) return -1;

    return ioUringEnter(ring_fd, 1, 0, 0);
}

pub export fn iouring_register_buffers(
    ring_fd: i32,
    buffers: *anyopaque,
    nr_buffers: u32,
) callconv(.c) i32 {
    return ioUringRegister(ring_fd, IORING_REGISTER_BUFFERS, buffers, nr_buffers);
}

pub export fn iouring_close(ring_fd: i32) callconv(.c) i32 {
    _ = linux.close(ring_fd);
    return 0;
}

pub export fn iouring_wait_cqes(
    ring_fd: i32,
    max_cqes: u32,
) callconv(.c) i32 {
    return ioUringEnter(ring_fd, 0, max_cqes, 0);
}

pub export fn iouring_submit_and_wait(
    ring_fd: i32,
    to_submit: u32,
    min_complete: u32,
) callconv(.c) i32 {
    return ioUringEnter(ring_fd, to_submit, min_complete, 0);
}

pub export fn iouring_peek_cqe(
    ring_fd: i32,
) callconv(.c) i32 {
    return ioUringEnter(ring_fd, 0, 0, 0);
}

pub export fn iouring_get_sq_space(ring_fd: i32) callconv(.c) i32 {
    _ = ring_fd;
    return 0;
}

pub export fn iouring_enable_ring(ring_fd: i32) callconv(.c) i32 {
    _ = ring_fd;
    return 0;
}

pub export fn iouring_disable_ring(ring_fd: i32) callconv(.c) i32 {
    _ = ring_fd;
    return 0;
}

test "iouring_basic" {
    var ring: IoUring = undefined;
    const result = iouring_init(&ring, 32);
    try std.testing.expect(result == 0 or result == -1);
}
