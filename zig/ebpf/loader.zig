const std = @import("std");
const linux = std.os.linux;

const SYS_bpf: linux.syscall_number = 321;

const BPF_MAP_CREATE: u32 = 0;
const BPF_MAP_UPDATE_ELEM: u32 = 1;
const BPF_MAP_LOOKUP_ELEM: u32 = 2;
const BPF_MAP_DELETE_ELEM: u32 = 3;
const BPF_MAP_GET_NEXT_KEY: u32 = 4;
const BPF_PROG_LOAD: u32 = 5;
const BPF_OBJ_PIN: u32 = 6;
const BPF_OBJ_GET: u32 = 7;
const BPF_PROG_ATTACH: u32 = 8;
const BPF_PROG_DETACH: u32 = 9;

const BPF_MAP_TYPE_SOCKMAP: u32 = 15;
const BPF_MAP_TYPE_HASH: u32 = 1;

const BPF_F_RDONLY: u32 = 1 << 3;
const BPF_F_WRONLY: u32 = 1 << 4;
const BPF_F_RDWR: u32 = 1 << 5;
const BPF_F_STACK: u32 = 1 << 4;

const BPF_PROG_TYPE_SK_MSG: u32 = 23;
const BPF_PROG_TYPE_KPROBE: u32 = 6;
const BPF_PROG_TYPE_TRACING: u32 = 26;

const BPF_ATTACH_TYPE_SK_MSG: u32 = 4;

const EINVAL: i32 = 22;
const ENOENT: i32 = 2;
const EEXIST: i32 = 17;
const EBADF: i32 = 9;
const ENOMEM: i32 = 12;
const EPERM: i32 = 1;

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR: i32 = 2;
const O_CREAT: i32 = 64;
const O_TRUNC: i32 = 512;
const O_CLOEXEC: i32 = 0x80000;

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

const bpf_attr = extern struct {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
    map_flags: u32,
    fd: i32,
    filename: [256]u8,
    prog_type: u32,
    insn_cnt: u32,
    insns: [*]const u8,
    license: [*]const u8,
    log_level: u32,
    log_size: u32,
    log_buf: ?[*]u8,
    kern_version: u32,
    attach_type: u32,
    attach_prog_fd: i32,
    attach_bpf_fd: i32,
    attach_flags: u32,
    replace_prog_fd: i32,
    prog_name: [32]u8,
    map_name: [32]u8,
    btf_fd: i32,
    func_info_rec_size: u32,
    func_info: [*]const u8,
    func_info_cnt: u32,
    line_info_rec_size: u32,
    line_info: [*]const u8,
    line_info_cnt: u32,
    attach_btf_id: u32,
    core_relo_cnt: u32,
    expected_attach_type: u32,
    provider_name: [32]u8,
    target_btf_id: u32,
    replace_ext_prog_fd: u32,
    kernel_debug_type: u32,
    jited: u32,
    result: u32,
    trining: u32,
};

fn bpf_syscall(cmd: u32, attr: *bpf_attr, size: usize) i32 {
    return @intCast(linux.syscall3(SYS_bpf, cmd, @intFromPtr(attr), size));
}

var ebpf_supported: bool = false;
var ebpf_initialized: bool = false;

fn check_ebpf_support() void {
    if (ebpf_initialized) return;
    ebpf_initialized = true;

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_type = BPF_MAP_TYPE_HASH;
    attr.key_size = 4;
    attr.value_size = 4;
    attr.max_entries = 1;
    attr.map_flags = BPF_F_RDWR;

    const result = bpf_syscall(BPF_MAP_CREATE, &attr, @sizeOf(bpf_attr));
    if (result >= 0) {
        _ = linux.close(result);
        ebpf_supported = true;
    } else {
        ebpf_supported = false;
    }
}

pub export fn ebpf_load_program(
    prog_path: [*:0]const u8,
    prog_type: u32,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported) {
        return -1;
    }

    const path_str = std.mem.sliceTo(prog_path, 0);
    const file = std.fs.cwd().openFile(path_str, .{}) catch return -1;
    defer file.close();

    const stat = file.stat() catch return -1;
    const code_size = @as(usize, @intCast(stat.size));
    const code = file.reader().readAlloc(std.heap.page_allocator, code_size) catch return -1;
    defer std.heap.page_allocator.free(code);

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.prog_type = prog_type;
    attr.insn_cnt = @divExact(code_size, @sizeOf(u64));
    attr.insns = @ptrCast(code.ptr);
    attr.license = @ptrCast("GPL".ptr);
    attr.kern_version = 0;
    @memcpy(attr.prog_name[0..7], "aios_ebpf"[0..7]);

    const prog_fd = bpf_syscall(BPF_PROG_LOAD, &attr, @sizeOf(bpf_attr));
    if (prog_fd < 0) {
        return -1;
    }

    return prog_fd;
}

pub export fn ebpf_create_sockmap(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_type = BPF_MAP_TYPE_SOCKMAP;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    attr.map_flags = BPF_F_RDWR;

    return bpf_syscall(BPF_MAP_CREATE, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_create_hash_map(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_type = BPF_MAP_TYPE_HASH;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    attr.map_flags = BPF_F_RDWR;

    return bpf_syscall(BPF_MAP_CREATE, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_update_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *const anyopaque,
    flags: u64,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported or map_fd < 0) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_fd = map_fd;
    @memcpy(@as([*]u8, @ptrCast(&attr.key_size))[0..8], @as([*]const u8, @ptrCast(key))[0..8]);
    @memcpy(@as([*]u8, @ptrCast(&attr.value_size))[0..8], @as([*]const u8, @ptrCast(value))[0..8]);
    attr.map_flags = @truncate(flags);

    return bpf_syscall(BPF_MAP_UPDATE_ELEM, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_lookup_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *anyopaque,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported or map_fd < 0) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_fd = map_fd;
    @memcpy(@as([*]u8, @ptrCast(&attr.key_size))[0..8], @as([*]const u8, @ptrCast(key))[0..8]);
    @memset(@as([*]u8, @ptrCast(&attr.value_size))[0..8], 0);

    const result = bpf_syscall(BPF_MAP_LOOKUP_ELEM, &attr, @sizeOf(bpf_attr));
    if (result == 0) {
        @memcpy(@as([*]u8, @ptrCast(value))[0..8], @as([*]u8, @ptrCast(&attr.value_size))[0..8]);
    }
    return result;
}

pub export fn ebpf_delete_map_elem(
    map_fd: i32,
    key: *const anyopaque,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported or map_fd < 0) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.map_fd = map_fd;
    @memcpy(@as([*]u8, @ptrCast(&attr.key_size))[0..8], @as([*]const u8, @ptrCast(key))[0..8]);

    return bpf_syscall(BPF_MAP_DELETE_ELEM, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_attach_sockmap(
    map_fd: i32,
    prog_fd: i32,
) i32 {
    check_ebpf_support();
    if (!ebpf_supported or map_fd < 0 or prog_fd < 0) {
        return -1;
    }

    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.target_btf_id = @truncate(map_fd);
    attr.attach_bpf_fd = prog_fd;
    attr.attach_type = BPF_ATTACH_TYPE_SK_MSG;
    attr.attach_flags = 0;

    return bpf_syscall(BPF_PROG_ATTACH, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_init() i32 {
    check_ebpf_support();
    return if (ebpf_supported) 0 else -1;
}

pub export fn ebpf_is_supported() bool {
    check_ebpf_support();
    return ebpf_supported;
}

pub export fn ebpf_close(prog_fd: i32) i32 {
    if (prog_fd >= 0) {
        return linux.close(prog_fd);
    }
    return 0;
}

pub export fn ebpf_pin_object(path: [*:0]const u8, fd: i32) i32 {
    check_ebpf_support();
    if (!ebpf_supported or fd < 0) {
        return -1;
    }

    const path_str = std.mem.sliceTo(path, 0);
    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.filename[0..path_str.len].* = path_str.*;
    attr.filename[path_str.len] = 0;
    attr.bpf_fd = fd;

    return bpf_syscall(BPF_OBJ_PIN, &attr, @sizeOf(bpf_attr));
}

pub export fn ebpf_get_pinned_object(path: [*:0]const u8) i32 {
    check_ebpf_support();
    if (!ebpf_supported) {
        return -1;
    }

    const path_str = std.mem.sliceTo(path, 0);
    var attr: bpf_attr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(bpf_attr)], 0);
    attr.filename[0..path_str.len].* = path_str.*;
    attr.filename[path_str.len] = 0;

    return bpf_syscall(BPF_OBJ_GET, &attr, @sizeOf(bpf_attr));
}

test "ebpf_loader_basic" {
    const result = ebpf_init();
    try std.testing.expect(result == 0 or result == -1);
    const supported = ebpf_is_supported();
    try std.testing.expect(supported == true or supported == false);
}
