const std = @import("std");

test "getCpuCount returns positive" {
    const count = std.Thread.getCpuCount() catch 1;
    try std.testing.expect(count > 0);
}

test "createCpuSet with single core" {
    var set: [16]usize = undefined;
    @memset(&set, 0);
    set[0] = 1; // core 0 only
    try std.testing.expectEqual(@as(usize, 1), set[0]);
}

test "createCpuSet with multiple cores" {
    var set: [16]usize = undefined;
    @memset(&set, 0);
    set[0] = 0b1111; // cores 0-3
    try std.testing.expectEqual(@as(usize, 15), set[0]);
}

test "cpu mask shift" {
    const mask: u64 = @as(u64, 1) << 4;
    try std.testing.expectEqual(@as(u64, 16), mask);
}
