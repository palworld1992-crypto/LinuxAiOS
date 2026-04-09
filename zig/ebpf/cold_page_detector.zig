const std = @import("std");
const linux = @import("std").os.linux;
const loader = @import("loader.zig");

pub const ColdPageEvent = extern struct {
    pid: u32,
    addr: u64,
    timestamp: u64,
    access_count: u32,
};

const MAX_EVENTS = 1024;
var g_events: [MAX_EVENTS]ColdPageEvent = undefined;
var g_event_count: u32 = 0;
var g_tracking_enabled: bool = false;
var g_last_sample_time: u64 = 0;

pub export fn ebpf_init_coldpage_detector(
    ring_buf_fd: *i32,
    prog_fd: *i32,
) callconv(.c) i32 {
    if (!loader.ebpf_is_supported()) {
        ring_buf_fd.* = -1;
        prog_fd.* = -1;
        return -1;
    }

    const map_fd = loader.ebpf_create_hash_map(8, 8, 4096);
    if (map_fd < 0) {
        ring_buf_fd.* = -1;
        prog_fd.* = -1;
        return -1;
    }

    ring_buf_fd.* = map_fd;
    prog_fd.* = map_fd;

    return 0;
}

pub export fn ebpf_attach_coldpage_detector(
    prog_fd: i32,
) callconv(.c) i32 {
    _ = prog_fd;
    return 0;
}

pub export fn ebpf_read_coldpage_events(
    ring_buf_fd: i32,
    events: [*]ColdPageEvent,
    max_events: usize,
) callconv(.c) i32 {
    if (ring_buf_fd < 0) {
        return read_from_userspace_ring(events, max_events);
    }

    var total_read: u32 = 0;
    var buf: [64]u8 = undefined;

    while (total_read < max_events) {
        const n = linux.read(ring_buf_fd, &buf, @sizeOf(@TypeOf(buf)));
        if (n < 0) break;
        if (n == 0) break;

        if (n >= @sizeOf(ColdPageEvent)) {
            const event_size = @sizeOf(ColdPageEvent);
            const num_events = @min(@as(usize, @intCast(n)) / event_size, max_events - total_read);

            var i: usize = 0;
            while (i < num_events) : (i += 1) {
                const event_ptr = @as([*]ColdPageEvent, @ptrCast(@alignCast(&buf)))[i];
                events[@intCast(total_read + i)] = event_ptr;
            }
            total_read += @intCast(num_events);
        }
    }

    if (total_read == 0) {
        return read_from_userspace_ring(events, max_events);
    }

    return @intCast(total_read);
}

fn read_from_userspace_ring(events: [*]ColdPageEvent, max_events: usize) i32 {
    const count = @min(g_event_count, @as(u32, @intCast(max_events)));
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        events[i] = g_events[i];
    }
    return @intCast(count);
}

pub export fn fallback_check_cold_pages(
    addr: usize,
    len: usize,
    cold_pages: [*]u64,
    max_cold: usize,
) callconv(.c) i32 {
    if (max_cold == 0) return 0;

    const page_size: usize = 4096;
    const num_pages = @divExact(len + page_size - 1, page_size);

    var cold_count: usize = 0;
    var page_idx: usize = 0;

    while (page_idx < num_pages and cold_count < max_cold) : (page_idx += 1) {
        const vaddr = addr + page_idx * page_size;

        var sample_data: [16]u8 = undefined;
        const test_fd = linux.open("/proc/self/mem", linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
        if (@as(isize, @bitCast(test_fd)) < 0) continue;

        _ = linux.pread(@as(i32, @intCast(test_fd)), &sample_data, sample_data.len, @intCast(vaddr));
        _ = linux.close(@as(i32, @intCast(test_fd)));

        var all_zero = true;
        for (sample_data) |b| {
            if (b != 0) {
                all_zero = false;
                break;
            }
        }

        if (all_zero) {
            cold_pages[cold_count] = vaddr;
            cold_count += 1;
        }
    }

    return @intCast(cold_count);
}

pub export fn cold_page_detector_enable_tracking() callconv(.c) i32 {
    g_tracking_enabled = true;
    g_event_count = 0;
    return 0;
}

pub export fn cold_page_detector_disable_tracking() callconv(.c) i32 {
    g_tracking_enabled = false;
    return 0;
}

pub export fn cold_page_detector_get_stats() callconv(.c) i32 {
    return @intCast(g_event_count);
}

pub export fn cold_page_detector_add_event(event: ColdPageEvent) callconv(.c) void {
    if (g_event_count < MAX_EVENTS) {
        g_events[g_event_count] = event;
        g_event_count += 1;
    }
}

pub export fn cold_page_detector_sample_memory(addr: u64, len: usize) callconv(.c) void {
    if (!g_tracking_enabled) return;

    const page_size: usize = 4096;
    const num_pages = (len + page_size - 1) / page_size;
    var ts: linux.timespec = undefined;
    _ = linux.clock_gettime(linux.CLOCK.REALTIME, &ts);
    const now: u64 = @as(u64, @intCast(ts.sec));

    if (now - g_last_sample_time < 10) return;
    g_last_sample_time = now;

    var i: usize = 0;
    while (i < num_pages and g_event_count < MAX_EVENTS) : (i += 1) {
        const vaddr = addr + i * page_size;
        var sample_buf: [16]u8 = undefined;

        const mem_fd = linux.open("/proc/self/mem", linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
        if (@as(isize, @bitCast(mem_fd)) < 0) continue;

        _ = linux.pread(@as(i32, @intCast(mem_fd)), &sample_buf, sample_buf.len, @intCast(vaddr));
        _ = linux.close(@as(i32, @intCast(mem_fd)));

        var access_count: u32 = 0;
        for (sample_buf) |b| {
            if (b != 0) access_count += 1;
        }

        if (access_count == 0) {
            g_events[g_event_count] = ColdPageEvent{
                .pid = 0,
                .addr = vaddr,
                .timestamp = @intCast(now),
                .access_count = 0,
            };
            g_event_count += 1;
        }
    }
}

test "cold_page_detector_basic" {
    try std.testing.expectEqual(@as(i32, 0), cold_page_detector_enable_tracking());
    try std.testing.expectEqual(@as(i32, 0), cold_page_detector_get_stats());
    try std.testing.expectEqual(@as(i32, 0), cold_page_detector_disable_tracking());
}
