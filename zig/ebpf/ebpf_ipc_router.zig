const std = @import("std");
const ebpf = @import("ebpf_loader.zig");
const posix = std.posix;
const linux = std.os.linux;

pub const RouterInfo = struct { prog_fd: posix.fd_t, map_fd: posix.fd_t };

/// Khởi tạo router với eBPF sockmap
pub fn init_router(prog_path: []const u8) !RouterInfo {
    // Lưu ý: load_bpf_program cần khớp với định nghĩa trong ebpf_loader.zig của bạn
    const prog_fd = try ebpf.load_bpf_program(.sk_msg, prog_path);

    // Key size cho SOCKMAP chuẩn thường là u32 (index)
    const map_fd = try ebpf.create_bpf_map(.sockmap, @sizeOf(u32), @sizeOf(u32), 1024);

    try ebpf.attach_sockmap_prog(map_fd, prog_fd);

    return RouterInfo{ .prog_fd = prog_fd, .map_fd = map_fd };
}

/// Update routing: key = source peer ID, value = socket fd
pub fn update_route(map_fd: posix.fd_t, src_peer: u32, dst_sock: u32) !void {
    // SOCKMAP key thường là u32. Nếu dùng u64 phải đảm bảo map tạo ra với key_size 8.
    // Truyền trực tiếp con trỏ vào hàm update của loader
    try ebpf.update_bpf_map(map_fd, &src_peer, &dst_sock);
}

/// Xóa route - Cập nhật theo chuẩn syscall Zig 0.15.2
pub fn remove_route(map_fd: posix.fd_t, src_peer: u32) !void {
    var attr: linux.BPF_ATTR = std.mem.zeroes(linux.BPF_ATTR);

    // Thiết lập thông số xóa element trong map
    attr.map_delete_elem = .{
        .map_fd = @intCast(map_fd), // @intCast trong 0.15.2 chỉ nhận 1 tham số
        .key = @intFromPtr(&src_peer),
    };

    // Zig 0.15.2 dùng syscall3 cho BPF (Opcode xóa là 3)
    const rc = linux.syscall3(.bpf, 3, @intFromPtr(&attr), @sizeOf(linux.BPF_ATTR));

    if (linux.getErrno(rc) != .SUCCESS) return error.DeleteFailed;
}
