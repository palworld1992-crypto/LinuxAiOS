const std = @import("std");

pub const BloomFilter = struct {
    bit_array: []u8,
    num_bits: usize,
    num_hashes: usize,
};

pub export fn bloom_create(
    expected_items: u32,
    false_positive_rate: f64,
) ?*BloomFilter {
    _ = expected_items;
    _ = false_positive_rate;
    return null;
}

fn bloom_hash_idx(filter: *BloomFilter, item: [*]const u8, len: usize, seed: u64) usize {
    _ = filter;
    _ = item;
    _ = len;
    _ = seed;
    return 0;
}

pub export fn bloom_add(
    filter: *BloomFilter,
    item: [*]const u8,
    len: usize,
) void {
    _ = filter;
    _ = item;
    _ = len;
}

pub export fn bloom_might_contain(
    filter: *BloomFilter,
    item: [*]const u8,
    len: usize,
) bool {
    _ = filter;
    _ = item;
    _ = len;
    return false;
}

pub export fn bloom_destroy(filter: *BloomFilter) void {
    _ = filter;
}

pub export fn bloom_clear(filter: *BloomFilter) void {
    _ = filter;
}

test "bloom_basic" {
    const filter = bloom_create(1000, 0.01);
    try std.testing.expect(filter == null);
}
