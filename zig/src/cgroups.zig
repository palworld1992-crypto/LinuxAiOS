const std = @import("std");

/// Enterprise-grade cgroup freezing
/// Đóng băng cgroup bằng cách ghi "1" vào file freezer.state hoặc cgroup.freeze
pub fn freeze_cgroup(path: [*:0]const u8) i32 {
    const path_slice = std.mem.span(path);
    
    // Sử dụng std.fs.cwd() là cách an toàn và đúng đắn trong Zig 0.15.x
    // Tham số thứ 2 là struct định nghĩa quyền truy cập
    var file = std.fs.cwd().openFile(path_slice, .{ .mode = .write_only }) catch |err| {
        std.debug.print("Error opening cgroup path {s}: {any}\n", .{ path_slice, err });
        return -1;
    };
    defer file.close();

    file.writeAll("1") catch |err| {
        std.debug.print("Error writing to cgroup: {any}\n", .{ err });
        return -1;
    };
    
    return 0;
}

/// Enterprise-grade cgroup thawing
/// Kích hoạt lại cgroup bằng cách ghi "0"
pub fn thaw_cgroup(path: [*:0]const u8) i32 {
    const path_slice = std.mem.span(path);
    
    var file = std.fs.cwd().openFile(path_slice, .{ .mode = .write_only }) catch |err| {
        std.debug.print("Error opening cgroup path {s}: {any}\n", .{ path_slice, err });
        return -1;
    };
    defer file.close();

    file.writeAll("0") catch |err| {
        std.debug.print("Error writing to cgroup: {any}\n", .{ err });
        return -1;
    };
    
    return 0;
}