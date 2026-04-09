const std = @import("std");
const build_options = @import("build_options"); // Nhận từ build.zig
const ebpf = @import("ebpf_loader.zig");
const cgroups = @import("cgroups.zig");
export fn zig_load_coldpage_program_default() i32 {
    // build_options.coldpage_obj sẽ trỏ thẳng vào file .o đã biên dịch
    const path = build_options.coldpage_obj;
    const fd = ebpf.load_bpf_program(path, .tracepoint) catch return -1;
    return @intCast(fd);
}

/// Tạo BPF Map (Hash, LRU, Sockmap...)
export fn zig_create_bpf_map(map_type: u32, key_size: u32, value_size: u32, max_entries: u32) i32 {
    const fd = ebpf.create_bpf_map(@enumFromInt(map_type), key_size, value_size, max_entries) catch |err| {
        std.debug.print("AIOS Map Creation Error: {any}\n", .{err});
        return -1;
    };
    return @intCast(fd);
}

/// Cập nhật Map từ Userspace (Dùng cho IPC hoặc cấu hình swap)
export fn zig_update_bpf_map(map_fd: i32, key_ptr: [*]const u8, key_len: usize, value_ptr: [*]const u8, value_len: usize) i32 {
    const key = key_ptr[0..key_len];
    const value = value_ptr[0..value_len];
    ebpf.update_bpf_map(map_fd, key, value) catch return -1;
    return 0;
}

/// Đính kèm chương trình verdict vào Sockmap (Phần IPC của AIOS)
export fn zig_set_sockmap_prog(map_fd: i32, prog_fd: i32) i32 {
    ebpf.set_sockmap_prog(map_fd, prog_fd) catch return -1;
    return 0;
}

/// Chuyên biệt cho Cold Page Tracking (Hệ thống Predictive Swap)
export fn zig_load_coldpage_program(obj_path: [*:0]const u8) i32 {
    const path = std.mem.span(obj_path);
    const fd = ebpf.load_bpf_program(path, .tracepoint) catch |err| {
        std.debug.print("AIOS Coldpage Load Error: {any}\n", .{err});
        return -1;
    };
    return @intCast(fd);
}

/// Kích hoạt giám sát Page Fault
export fn zig_attach_coldpage_program(prog_fd: i32) i32 {
    // Gắn vào tracepoint: exceptions/page_fault_user
    ebpf.attach_tracepoint(prog_fd, "exceptions", "page_fault_user") catch return -1;
    return 0;
}

/// Điều khiển Cgroups (Freezer)
export fn zig_cgroup_freeze(path: [*:0]const u8) i32 {
    return cgroups.freeze_cgroup(path);
}

export fn zig_cgroup_thaw(path: [*:0]const u8) i32 {
    return cgroups.thaw_cgroup(path);
}

/// Placeholder cho logic nén dữ liệu Enterprise
export fn zig_compress_and_store(pid: u32, addr: u64, len: usize, path: [*:0]const u8) i32 {
    _ = pid;
    _ = addr;
    _ = len;
    _ = path;
    // TODO: Tích hợp thư viện nén (như Zstd hoặc LZ4) cho AIOS Swap Core
    return 0;
}

/// Tương thích ngược (Legacy)
export fn zig_ebpf_load_legacy(program_path: [*:0]const u8) i32 {
    const path = std.mem.span(program_path);
    const fd = ebpf.load_bpf_program(path, .sockmap) catch return -1;
    return @intCast(fd);
}
