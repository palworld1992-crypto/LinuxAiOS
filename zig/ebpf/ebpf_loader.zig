const std = @import("std");
const posix = std.posix;
const linux = std.os.linux;

// --- Khai báo hằng số BPF thiếu trong std.os.linux của Zig 0.15.2 ---
const BPF_MAP_CREATE = 0;
const BPF_PROG_LOAD = 5;
const BPF_PROG_ATTACH = 8;
const BPF_MAP_UPDATE_ELEM = 2;

pub const BpfProgType = enum(u32) {
    tracepoint = 2,
    sockmap = 19,
    sk_msg = 23,
};

pub const BpfMapType = enum(u32) {
    hash = 1,
    array = 2,
    lru_hash = 12,
    sockmap = 15,
};

/// Enterprise Map Creation - Zig 0.15.2 chuẩn
pub fn create_bpf_map(map_type: BpfMapType, key_size: u32, value_size: u32, max_entries: u32) !posix.fd_t {
    // Sử dụng cấu trúc linux.BPF_ATTR (union) thay cho các attr riêng lẻ cũ
    var attr: linux.BPF_ATTR = std.mem.zeroes(linux.BPF_ATTR);
    attr.map_create = .{
        .map_type = @intFromEnum(map_type),
        .key_size = key_size,
        .value_size = value_size,
        .max_entries = max_entries,
    };

    // Gọi trực tiếp syscall BPF
    const ret = linux.syscall3(.bpf, BPF_MAP_CREATE, @intFromPtr(&attr), @sizeOf(linux.BPF_ATTR));
    if (linux.getErrno(ret) != .SUCCESS) return error.MapCreationFailed;
    
    return @intCast(ret);
}

/// Load BPF Program (Zig 0.15.2)
pub fn load_bpf_program(prog_type: BpfProgType, insns: []const linux.bpf_insn, license: [:0]const u8) !posix.fd_t {
    var attr: linux.BPF_ATTR = std.mem.zeroes(linux.BPF_ATTR);
    
    // Cấp phát log buffer để Verifier báo lỗi nếu load thất bại
    const log_buf = try std.heap.page_allocator.alloc(u8, 8192);
    defer std.heap.page_allocator.free(log_buf);
    @memset(log_buf, 0);

    attr.prog_load = .{
        .prog_type = @intFromEnum(prog_type),
        .insn_cnt = @intCast(insns.len),
        .insns = @intFromPtr(insns.ptr),
        .license = @intFromPtr(license.ptr),
        .log_level = 1,
        .log_size = @intCast(log_buf.len),
        .log_buf = @intFromPtr(log_buf.ptr),
    };

    const ret = linux.syscall3(.bpf, BPF_PROG_LOAD, @intFromPtr(&attr), @sizeOf(linux.BPF_ATTR));
    
    if (linux.getErrno(ret) != .SUCCESS) {
        std.debug.print("BPF Verifier Log:\n{s}\n", .{log_buf});
        return error.ProgLoadFailed;
    }
    return @intCast(ret);
}

/// Attach sk_msg program to a sockmap
pub fn attach_sockmap_prog(map_fd: posix.fd_t, prog_fd: posix.fd_t) !void {
    var attr: linux.BPF_ATTR = std.mem.zeroes(linux.BPF_ATTR);
    attr.prog_attach = .{
        .target_fd = @intCast(map_fd),
        .attach_bpf_fd = @intCast(prog_fd),
        .attach_type = 16, // BPF_SK_MSG_VERDICT
        .attach_flags = 0,
    };

    const ret = linux.syscall3(.bpf, BPF_PROG_ATTACH, @intFromPtr(&attr), @sizeOf(linux.BPF_ATTR));
    if (linux.getErrno(ret) != .SUCCESS) return error.AttachFailed;
}

/// Update map element
pub fn update_bpf_map(map_fd: posix.fd_t, key: *const anyopaque, value: *const anyopaque) !void {
    var attr: linux.BPF_ATTR = std.mem.zeroes(linux.BPF_ATTR);
    attr.map_update_elem = .{
        .map_fd = @intCast(map_fd),
        .key = @intFromPtr(key),
        .value = @intFromPtr(value),
        .flags = 0, // BPF_ANY
    };

    const ret = linux.syscall3(.bpf, BPF_MAP_UPDATE_ELEM, @intFromPtr(&attr), @sizeOf(linux.BPF_ATTR));
    if (linux.getErrno(ret) != .SUCCESS) return error.UpdateFailed;
}