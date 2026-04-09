const std = @import("std");
const linux = @import("std").os.linux;

/// Bloom filter module - probabilistic set membership testing
/// Uses multiple hash functions for accurate filtering
const MAX_BITS = 1024 * 1024;

pub const BloomFilter = struct {
    bits: [MAX_BITS / 8]u8,
    num_bits: u32,
    num_hashes: u32,
    item_count: u32,
    expected_items: u32,
    false_positive_rate: f64,
};

fn optimalHashCount(num_bits: u32, expected_items: u32) u32 {
    if (expected_items == 0) return 1;
    const ratio = @as(f64, @floatFromInt(num_bits)) / @as(f64, @floatFromInt(expected_items));
    const k = ratio * std.math.ln2;
    return @max(1, @as(u32, @intFromFloat(k)));
}

fn optimalBitCount(expected_items: u32, fpr: f64) u32 {
    if (fpr <= 0.0 or fpr >= 1.0) return MAX_BITS;
    const ln_fpr = std.math.log(f64, std.math.e, fpr);
    const bits = -@as(f64, @floatFromInt(expected_items)) * ln_fpr / (std.math.ln2 * std.math.ln2);
    const capped = @min(@as(u64, @intFromFloat(bits)), MAX_BITS);
    return @as(u32, @intCast(capped));
}

fn hashFn(data: []const u8, seed: u32) u32 {
    var h: u32 = seed;
    for (data) |b| {
        h = h *% 31 +% @as(u32, b);
        h ^= h >> 13;
        h = h *% 0x5bd1e995;
        h ^= h >> 15;
    }
    return h;
}

fn setBit(filter: *BloomFilter, bit: u32) void {
    const idx = bit / 8;
    const offset = @as(u3, @intCast(bit % 8));
    filter.bits[idx] |= @as(u8, 1) << offset;
}

fn getBit(filter: *const BloomFilter, bit: u32) bool {
    const idx = bit / 8;
    const offset = @as(u3, @intCast(bit % 8));
    return (filter.bits[idx] >> offset) & 1 == 1;
}

pub export fn bloom_create(
    expected_items: u32,
    false_positive_rate: f64,
) callconv(.c) ?*BloomFilter {
    if (expected_items == 0) return null;
    if (false_positive_rate <= 0.0 or false_positive_rate >= 1.0) return null;

    const num_bits = optimalBitCount(expected_items, false_positive_rate);
    const num_hashes = optimalHashCount(num_bits, expected_items);

    const filter = std.heap.page_allocator.create(BloomFilter) catch return null;
    @memset(filter.bits[0..], 0);
    filter.num_bits = num_bits;
    filter.num_hashes = num_hashes;
    filter.item_count = 0;
    filter.expected_items = expected_items;
    filter.false_positive_rate = false_positive_rate;

    return filter;
}

pub export fn bloom_add(
    filter: *BloomFilter,
    item: [*]const u8,
    len: usize,
) callconv(.c) void {
    const data = item[0..len];
    var i: u32 = 0;
    while (i < filter.num_hashes) : (i += 1) {
        const h = hashFn(data, i *% 0x9e3779b9);
        const bit = h % filter.num_bits;
        setBit(filter, bit);
    }
    filter.item_count += 1;
}

pub export fn bloom_might_contain(
    filter: *BloomFilter,
    item: [*]const u8,
    len: usize,
) callconv(.c) bool {
    const data = item[0..len];
    var i: u32 = 0;
    while (i < filter.num_hashes) : (i += 1) {
        const h = hashFn(data, i *% 0x9e3779b9);
        const bit = h % filter.num_bits;
        if (!getBit(filter, bit)) return false;
    }
    return true;
}

pub export fn bloom_destroy(filter: *BloomFilter) callconv(.c) void {
    std.heap.page_allocator.destroy(filter);
}

pub export fn bloom_clear(filter: *BloomFilter) callconv(.c) void {
    @memset(filter.bits[0..], 0);
    filter.item_count = 0;
}

pub export fn bloom_get_size(filter: *BloomFilter) callconv(.c) usize {
    return filter.num_bits;
}

pub export fn bloom_get_count(filter: *BloomFilter) callconv(.c) u32 {
    return filter.item_count;
}

pub export fn bloom_get_expected_fpr(filter: *BloomFilter) callconv(.c) f64 {
    return filter.false_positive_rate;
}

test "bloom_basic" {
    const filter = bloom_create(1000, 0.01);
    try std.testing.expect(filter != null);
    try std.testing.expectEqual(@as(u32, 0), bloom_get_count(filter.?));

    const item = "hello";
    bloom_add(filter.?, item, item.len);
    try std.testing.expectEqual(@as(u32, 1), bloom_get_count(filter.?));
    try std.testing.expect(bloom_might_contain(filter.?, item, item.len));

    const not_item = "world";
    try std.testing.expect(!bloom_might_contain(filter.?, not_item, not_item.len));

    bloom_destroy(filter.?);
}

test "bloom_clear" {
    const filter = bloom_create(100, 0.05);
    try std.testing.expect(filter != null);

    const item = "test";
    bloom_add(filter.?, item, item.len);
    try std.testing.expectEqual(@as(u32, 1), bloom_get_count(filter.?));

    bloom_clear(filter.?);
    try std.testing.expectEqual(@as(u32, 0), bloom_get_count(filter.?));

    bloom_destroy(filter.?);
}
