const std = @import("std");
const types = @import("zig_types");

test "version is valid" {
    try std.testing.expect(types.version.len > 0);
}

test "BloomFilter size" {
    try std.testing.expect(@sizeOf(types.BloomFilter) > 0);
    const bf = types.BloomFilter{
        .bits = undefined,
        .num_bits = 131072 * 8,
        .num_hashes = 0,
        .item_count = 0,
        .expected_items = 1000,
        .false_positive_rate = 0.01,
    };
    try std.testing.expectEqual(@as(u32, 0), bf.item_count);
}

test "RouteEntry struct" {
    const entry = types.RouteEntry{
        .src_peer = 1,
        .dst_sock = 2,
        .weight = 100,
        .urgency = 50,
        .ring_buffer_fd = -1,
        .active = true,
    };
    try std.testing.expect(entry.active);
    try std.testing.expectEqual(@as(u32, 1), entry.src_peer);
}

test "ColdPageEvent struct" {
    const event = types.ColdPageEvent{
        .pid = 1234,
        .addr = 0x1000,
        .timestamp = 1000000,
        .access_count = 0,
    };
    try std.testing.expectEqual(@as(u32, 1234), event.pid);
    try std.testing.expectEqual(@as(u32, 0), event.access_count);
}

test "RouteStats struct" {
    const stats = types.RouteStats{
        .total_packets = 0,
        .high_urgency = 0,
        .medium_urgency = 0,
        .low_urgency = 0,
        .dropped = 0,
        .forwarded = 0,
        .hebbian_updates = 0,
    };
    try std.testing.expectEqual(@as(u64, 0), stats.total_packets);
}

test "IoUring struct" {
    const ring = types.IoUring{
        .ring_fd = -1,
        .ring_size = 4096,
        .mapped = false,
    };
    try std.testing.expect(!ring.mapped);
    try std.testing.expectEqual(@as(i32, -1), ring.ring_fd);
}
