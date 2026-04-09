const std = @import("std");

pub fn criu_dump(pid: i32, image_dir: [*:0]const u8) i32 {
    var child = std.ChildProcess.init(&[_][]const u8{
        "criu", "dump", "-t", @as([*:0]const u8, @ptrCast(&pid)), "-D", image_dir, "-j"
    }, std.heap.page_allocator) catch return -1;
    defer child.deinit();
    const term = child.wait() catch return -2;
    return @intFromBool(term.exited());
}