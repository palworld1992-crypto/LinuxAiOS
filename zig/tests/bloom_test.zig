const std = @import("std");
const bloom = @import("zig_bloom");

test "bloom filter create and add" {
    var filter: bloom.BloomFilter = undefined;
    @memset(&filter.bits, 0);
    filter.num_bits = 1024;
    filter.num_hashes = 4;
    filter.item_count = 0;
    filter.expected_items = 100;
    filter.false_positive_rate = 0.01;

    const data = "hello world";
    bloom.bloom_add(&filter, data.ptr, data.len);
    try std.testing.expectEqual(@as(u32, 1), filter.item_count);
}

test "bloom filter empty before add" {
    var filter: bloom.BloomFilter = undefined;
    @memset(&filter.bits, 0);
    filter.num_bits = 1024;
    filter.num_hashes = 4;
    filter.item_count = 0;
    filter.expected_items = 100;
    filter.false_positive_rate = 0.01;

    try std.testing.expectEqual(@as(u32, 0), filter.item_count);
}

test "bloom create null on invalid input" {
    const result = bloom.bloom_create(0, 0.01);
    try std.testing.expect(result == null);
}
