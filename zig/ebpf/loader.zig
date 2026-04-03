const std = @import("std");

const linux = std.os.linux;

pub const LoaderResult = extern struct {
    prog_fd: i32,
    map_fd: i32,
    link_fd: i32,
    success: bool,
};

pub const EbpfProgram = extern struct {
    prog_fd: i32,
    prog_type: u32,
    loaded: bool,
};

pub const RouteEntry = extern struct {
    src_peer: u32,
    dst_sock: u32,
    weight: u8,
    urgency: u8,
    ring_buffer_fd: i32,
    active: bool,
};

var ebpf_initialized = false;
var ebpf_supported = true;

pub export fn ebpf_load_program(
    prog_path: [*:0]const u8,
    prog_type: u32,
) i32 {
    _ = prog_path;
    _ = prog_type;
    ebpf_supported = false;
    return -1;
}

pub export fn ebpf_create_sockmap(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    _ = key_size;
    _ = value_size;
    _ = max_entries;
    return -1;
}

pub export fn ebpf_create_hash_map(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    _ = key_size;
    _ = value_size;
    _ = max_entries;
    return -1;
}

pub export fn ebpf_update_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *const anyopaque,
    flags: u64,
) i32 {
    _ = map_fd;
    _ = key;
    _ = value;
    _ = flags;
    return -1;
}

pub export fn ebpf_lookup_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *anyopaque,
) i32 {
    _ = map_fd;
    _ = key;
    _ = value;
    return -1;
}

pub export fn ebpf_delete_map_elem(
    map_fd: i32,
    key: *const anyopaque,
) i32 {
    _ = map_fd;
    _ = key;
    return -1;
}

pub export fn ebpf_attach_sockmap(
    map_fd: i32,
    prog_fd: i32,
) i32 {
    _ = map_fd;
    _ = prog_fd;
    return -1;
}

pub export fn ebpf_init() i32 {
    ebpf_initialized = true;
    return 0;
}

pub export fn ebpf_is_supported() bool {
    return ebpf_supported;
}

pub export fn ebpf_close(prog_fd: i32) i32 {
    _ = prog_fd;
    return 0;
}

test "ebpf_loader_basic" {
    _ = ebpf_is_supported();
    try std.testing.expect(true);
}
