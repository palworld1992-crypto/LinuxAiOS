const std = @import("std");
const linux = std.os.linux;

pub const IoUring = extern struct {
    ring_fd: i32,
    ring_size: u32,
    mapped: bool,
};

const SYS_io_uring_setup: linux.syscall_number = 425;
const SYS_io_uring_enter: linux.syscall_number = 426;
const SYS_io_uring_register: linux.syscall_number = 427;

const IORING_SETUP_IOPOLL: u32 = 1 << 0;
const IORING_SETUP_SQPOLL: u32 = 1 << 1;
const IORING_SETUP_SQ_AFF: u32 = 1 << 2;
const IORING_SETUP_CQSIZE: u32 = 1 << 3;
const IORING_SETUP_CLAMP: u32 = 1 << 4;
const IORING_SETUP_ATTACH_WQ: u32 = 1 << 5;
const IORING_SETUP_R_DISABLED: u32 = 1 << 6;

const IORING_OP_READV: u32 = 0;
const IORING_OP_WRITEV: u32 = 1;
const IORING_OP_FSYNC: u32 = 2;
const IORING_OP_READ_FIXED: u32 = 5;
const IORING_OP_WRITE_FIXED: u32 = 6;
const IORING_OP_OPENAT: u32 = 7;

const IOSQE_FIXED_FILE: u32 = 1 << 0;
const IOSQE_IO_DRAIN: u32 = 1 << 1;
const IOSQE_IO_LINK: u32 = 1 << 2;
const IOSQE_IO_HARDLINK: u32 = 1 << 3;
const IOSQE_ASYNC: u32 = 1 << 4;
const IOSQE_BUFFER_SELECT: u32 = 1 << 5;

const IORING_ENTER_GETEVENTS: u32 = 1 << 0;
const IORING_ENTER_SQ_WAKEUP: u32 = 1 << 1;
const IORING_ENTER_SQ_WAIT: u32 = 1 << 2;
const IORING_ENTER_EXT_ARG: u32 = 1 << 3;

const IORING_REGISTER_BUFFERS: u32 = 0;
const IORING_UNREGISTER_BUFFERS: u32 = 1;
const IORING_REGISTER_FILES: u32 = 2;
const IORING_UNREGISTER_FILES: u32 = 3;

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR: i32 = 2;

fn io_uring_setup(entries: u32, params: *linux.io_uring_params) i32 {
    return @intCast(linux.syscall2(SYS_io_uring_setup, entries, @intFromPtr(params)));
}

fn io_uring_enter(fd: i32, to_submit: u32, min_complete: u32, flags: u32, sig: ?*linux.siginfo_t) i32 {
    return @intCast(linux.syscall5(SYS_io_uring_enter, @intCast(fd), to_submit, min_complete, flags, @intFromPtr(sig)));
}

fn io_uring_register(fd: i32, opcode: u32, arg: *const anyopaque, nr_args: u32) i32 {
    return @intCast(linux.syscall4(SYS_io_uring_register, @intCast(fd), opcode, @intFromPtr(arg), nr_args));
}

fn mmap_ring(size: usize, fd: i32) ?[*]u8 {
    const addr = linux.mmap(null, size, linux.PROT_READ | linux.PROT_WRITE, linux.MAP_SHARED | linux.MAP_POPULATE, fd, 0);
    if (addr == linux.MAP_FAILED) {
        return null;
    }
    return @as([*]u8, @ptrCast(addr));
}

pub export fn iouring_init(
    ring: *IoUring,
    entries: u32,
) i32 {
    var params: linux.io_uring_params = undefined;
    @memset(@as([*]u8, @ptrCast(&params))[0..@sizeOf(linux.io_uring_params)], 0);

    const fd = io_uring_setup(entries, &params);
    if (fd < 0) {
        ring.* = IoUring{
            .ring_fd = -1,
            .ring_size = 0,
            .mapped = false,
        };
        return fd;
    }

    const ring_size = params.sq_ring_bytes;
    const mapped = mmap_ring(ring_size, fd);

    if (mapped == null) {
        _ = linux.close(fd);
        ring.* = IoUring{
            .ring_fd = -1,
            .ring_size = 0,
            .mapped = false,
        };
        return -1;
    }

    ring.* = IoUring{
        .ring_fd = fd,
        .ring_size = ring_size,
        .mapped = true,
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
    if (ring_fd < 0) {
        return -1;
    }

    var sqe: linux.io_uring_sqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(linux.io_uring_sqe)], 0);

    sqe.opcode = IORING_OP_READV;
    sqe.fd = fd;
    sqe.addr = @intFromPtr(buf);
    sqe.len = @intCast(len);
    sqe.off = offset;
    sqe.user_data = user_data;
    sqe.flags = IOSQE_FIXED_FILE;

    const submitted = io_uring_enter(ring_fd, 1, 0, IORING_ENTER_SQ_WAKEUP, null);
    return @intCast(submitted);
}

pub export fn iouring_submit_write(
    ring_fd: i32,
    fd: i32,
    buf: [*]const u8,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    if (ring_fd < 0) {
        return -1;
    }

    var sqe: linux.io_uring_sqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(linux.io_uring_sqe)], 0);

    sqe.opcode = IORING_OP_WRITEV;
    sqe.fd = fd;
    sqe.addr = @intFromPtr(buf);
    sqe.len = @intCast(len);
    sqe.off = offset;
    sqe.user_data = user_data;
    sqe.flags = IOSQE_FIXED_FILE;

    const submitted = io_uring_enter(ring_fd, 1, 0, IORING_ENTER_SQ_WAKEUP, null);
    return @intCast(submitted);
}

pub export fn iouring_submit_openat(
    ring_fd: i32,
    dirfd: i32,
    path: [*:0]const u8,
    flags: i32,
    mode: u32,
    user_data: u64,
) i32 {
    if (ring_fd < 0) {
        return -1;
    }

    var sqe: linux.io_uring_sqe = undefined;
    @memset(@as([*]u8, @ptrCast(&sqe))[0..@sizeOf(linux.io_uring_sqe)], 0);

    sqe.opcode = IORING_OP_OPENAT;
    sqe.fd = dirfd;
    sqe.addr = @intFromPtr(path);
    sqe.len = @intCast(flags);
    sqe.off = @bitCast(@as(i64, @intCast(mode)));
    sqe.user_data = user_data;

    const submitted = io_uring_enter(ring_fd, 1, 0, IORING_ENTER_SQ_WAKEUP, null);
    return @intCast(submitted);
}

pub export fn iouring_register_buffers(
    ring_fd: i32,
    buffers: *anyopaque,
    nr_buffers: u32,
) i32 {
    if (ring_fd < 0) {
        return -1;
    }

    return io_uring_register(ring_fd, IORING_REGISTER_BUFFERS, buffers, nr_buffers);
}

pub export fn iouring_close(ring_fd: i32) i32 {
    if (ring_fd >= 0) {
        return linux.close(ring_fd);
    }
    return 0;
}

pub export fn iouring_wait_cqes(
    ring_fd: i32,
    max_cqes: u32,
) i32 {
    if (ring_fd < 0) {
        return -1;
    }

    return io_uring_enter(ring_fd, 0, max_cqes, IORING_ENTER_GETEVENTS, null);
}

test "iouring_basic" {
    var ring: IoUring = undefined;
    const result = iouring_init(&ring, 32, 0);
    try std.testing.expect(result == 0 or result == -1);
}
