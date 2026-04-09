const std = @import("std");

pub fn load_ebpf_program(path: [*:0]const u8) i32 {
    const path_slice = std.mem.span(path);
    const file = std.fs.cwd().openFile(path_slice, .{ .mode = .read_only }) catch return -1;
    defer file.close();

    // Logic xử lý nội dung file ở đây
    return 0;
}