const std = @import("std");
const types = @import("liblinux_zig_types.zig");

const MAX_BITS = 1024 * 1024;

fn bloomHashFn(data: []const u8, seed: u32) u32 {
    var h: u32 = seed;
    for (data) |b| {
        h = h *% 31 +% @as(u32, b);
        h ^= h >> 13;
        h = h *% 0x5bd1e995;
        h ^= h >> 15;
    }
    return h;
}

pub export fn liblinux_zig_bloom_create(expected_items: u32, false_positive_rate: f64) callconv(.c) ?*anyopaque {
    if (expected_items == 0) return null;
    if (false_positive_rate <= 0.0 or false_positive_rate >= 1.0) return null;
    const ln_fpr = std.math.log(f64, std.math.e, false_positive_rate);
    const num_bits_f = -@as(f64, @floatFromInt(expected_items)) * ln_fpr / (std.math.ln2 * std.math.ln2);
    const num_bits: u32 = @min(@as(u32, @intFromFloat(num_bits_f)), MAX_BITS);
    const ratio = @as(f64, @floatFromInt(num_bits)) / @as(f64, @floatFromInt(expected_items));
    const num_hashes = @max(@as(u32, 1), @as(u32, @intFromFloat(ratio * std.math.ln2)));

    const filter = std.heap.page_allocator.create(types.BloomFilter) catch return null;
    @memset(filter.bits[0..], 0);
    filter.num_bits = num_bits;
    filter.num_hashes = num_hashes;
    filter.item_count = 0;
    filter.expected_items = expected_items;
    filter.false_positive_rate = false_positive_rate;
    return filter;
}

pub export fn liblinux_zig_bloom_add(filter: *anyopaque, item: [*]const u8, len: usize) callconv(.c) void {
    const f: *types.BloomFilter = @ptrCast(@alignCast(filter));
    const data = item[0..len];
    var i: u32 = 0;
    while (i < f.num_hashes) : (i += 1) {
        const h = bloomHashFn(data, i *% 0x9e3779b9);
        const bit = h % f.num_bits;
        const idx = bit / 8;
        const offset = @as(u3, @intCast(bit % 8));
        f.bits[idx] |= @as(u8, 1) << offset;
    }
    f.item_count += 1;
}

pub export fn liblinux_zig_bloom_might_contain(filter: *anyopaque, item: [*]const u8, len: usize) callconv(.c) bool {
    const f: *types.BloomFilter = @ptrCast(@alignCast(filter));
    const data = item[0..len];
    var i: u32 = 0;
    while (i < f.num_hashes) : (i += 1) {
        const h = bloomHashFn(data, i *% 0x9e3779b9);
        const bit = h % f.num_bits;
        const idx = bit / 8;
        const offset = @as(u3, @intCast(bit % 8));
        if ((f.bits[idx] >> offset) & 1 == 0) return false;
    }
    return true;
}

pub export fn liblinux_zig_bloom_destroy(filter: *anyopaque) callconv(.c) void {
    std.heap.page_allocator.destroy(@as(*types.BloomFilter, @ptrCast(@alignCast(filter))));
}

pub export fn liblinux_zig_bloom_clear(filter: *anyopaque) callconv(.c) void {
    const f: *types.BloomFilter = @ptrCast(@alignCast(filter));
    @memset(f.bits[0..], 0);
    f.item_count = 0;
}
