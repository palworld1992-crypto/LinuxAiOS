const std = @import("std");

pub fn main() !void {
    std.debug.print("All your {s} are belong to us.\n", .{"codebase"});

    const gpa = std.heap.page_allocator;
    const args = try std.process.argsAlloc(gpa);
    defer std.process.argsFree(gpa, args);

    for (args) |arg| {
        std.log.info("arg: {s}", .{arg});
    }

    try std.io.getStdOut().writeAll("Hello from Zig 0.16!\n");
}

test "simple test" {
    const gpa = std.testing.allocator;
    var list = std.ArrayList(i32).init(gpa);
    defer list.deinit();
    try list.append(42);
    try std.testing.expectEqual(@as(i32, 42), list.pop());
}
