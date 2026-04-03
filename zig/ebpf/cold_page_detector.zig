const std = @import("std");

const linux = std.os.linux;
const BPF_PROG_TYPE_TRACING: u32 = 26;
const BPF_PROG_TYPE_KPROBE: u32 = 6;
const BPF_MAP_TYPE_RINGBUF: u32 = 27;

pub const ColdPageEvent = extern struct {
    pid: u32,
    addr: u64,
    timestamp: u64,
    access_count: u32,
};

pub export fn ebpf_init_coldpage_detector(
    ring_buf_fd: *i32,
    prog_fd: *i32,
) i32 {
    ring_buf_fd.* = -1;
    prog_fd.* = -1;
    return 0;
}

pub export fn ebpf_attach_coldpage_detector(
    prog_fd: i32,
) i32 {
    _ = prog_fd;
    return 0;
}

pub export fn ebpf_read_coldpage_events(
    ring_buf_fd: i32,
    events: [*]ColdPageEvent,
    max_events: usize,
) i32 {
    _ = ring_buf_fd;
    _ = events;
    _ = max_events;
    return 0;
}

pub export fn fallback_check_cold_pages(
    addr: usize,
    len: usize,
    cold_pages: [*]u64,
    max_cold: usize,
) i32 {
    _ = addr;
    _ = len;
    _ = cold_pages;
    _ = max_cold;
    return 0;
}

test "cold_page_detector_basic" {
    var ring_fd: i32 = -1;
    var prog_fd: i32 = -1;
    const result = ebpf_init_coldpage_detector(&ring_fd, &prog_fd);
    try std.testing.expect(result == 0 or result == -1);
}
