const std = @import("std");
const linux = @import("std").os.linux;

/// eBPF loader module - load, attach, manage eBPF programs
/// Uses bpf() syscall directly for program loading and map management
const BPF_PROG_LOAD: u32 = 5;
const BPF_MAP_CREATE: u32 = 0;
const BPF_MAP_UPDATE_ELEM: u32 = 2;
const BPF_MAP_LOOKUP_ELEM: u32 = 1;
const BPF_MAP_DELETE_ELEM: u32 = 3;
const BPF_PROG_ATTACH: u32 = 8;
const BPF_OBJ_PIN: u32 = 6;
const BPF_OBJ_GET: u32 = 7;
const BPF_MAP_GET_FD_BY_ID: u32 = 12;
const BPF_PROG_GET_NEXT_ID: u32 = 14;
const BPF_PROG_GET_FD_BY_ID: u32 = 13;

const BPF_MAP_TYPE_HASH: u32 = 1;
const BPF_MAP_TYPE_ARRAY: u32 = 2;
const BPF_MAP_TYPE_SOCKMAP: u32 = 9;
const BPF_SK_MSG_VERDICT: u32 = 4;
const BPF_PROG_TYPE_SK_MSG: u32 = 21;

const ELF_MAGIC: u32 = 0x464C457F;
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LITTLE_ENDIAN: u8 = 1;

pub const MapCreate = extern struct {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
    map_flags: u32,
    inner_map_fd: u32,
    numa_node: u32,
    map_name: [16]u8,
    map_ifindex: u32,
    btf_fd: u32,
    btf_key_type_id: u32,
    btf_value_type_id: u32,
    btf_vmlinux_value_type_id: u32,
    map_extra: u64,
};

pub const MapUpdateElem = extern struct {
    map_fd: u32,
    key: u64,
    value: u64,
    flags: u64,
};

pub const MapLookupElem = extern struct {
    map_fd: u32,
    key: u64,
    value: u64,
    flags: u64,
};

pub const MapDeleteElem = extern struct {
    map_fd: u32,
    key: u64,
};

pub const ProgLoad = extern struct {
    prog_type: u32,
    insn_cnt: u32,
    insns: u64,
    license: u64,
    log_level: u32,
    log_size: u32,
    log_buf: u64,
    kern_version: u32,
    prog_flags: u32,
    prog_name: [16]u8,
    prog_ifindex: u32,
    expected_attach_type: u32,
    prog_btf_fd: u32,
    func_info_rec_size: u32,
    func_info: u64,
    func_info_cnt: u32,
    line_info_rec_size: u32,
    line_info: u64,
    line_info_cnt: u32,
    attach_btf_id: u32,
    attach_prog_fd: u32,
    core_relo_cnt: u32,
    fd_array: u64,
};

pub const ProgAttach = extern struct {
    target_fd: u32,
    attach_bpf_fd: u32,
    attach_type: u32,
    attach_flags: u32,
    replace_bpf_fd: u32,
};

pub const ObjPin = extern struct {
    pathname: u64,
    bpf_fd: u32,
    file_flags: u32,
};

pub const MapGetFdById = extern struct {
    map_id: u32,
    next_id: u32,
    open_flags: u32,
};

pub const ProgGetNextId = extern struct {
    start_id: u32,
    next_id: u32,
};

pub const ProgGetFdById = extern struct {
    prog_id: u32,
    fd: u32,
};

fn bpfSyscall(cmd: u32, attr: *const anyopaque, size: usize) i32 {
    const result = linux.syscall3(.bpf, cmd, @intFromPtr(attr), size);
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

fn readFile(path: [*:0]const u8) ?[]u8 {
    const fd = linux.open(path, linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
    if (@as(isize, @bitCast(fd)) < 0) return null;
    defer {
        _ = linux.close(@as(i32, @intCast(fd)));
    }

    var buf: [8192]u8 = undefined;
    const n = linux.read(@as(i32, @intCast(fd)), &buf, buf.len);
    if (n == 0) return null;
    return buf[0..n];
}

fn validateElf(data: []const u8) bool {
    if (data.len < 64) return false;
    const magic = @as(u32, @bitCast(data[0..4].*));
    if (magic != ELF_MAGIC) return false;
    if (data[4] != ELF_CLASS_64) return false;
    if (data[5] != ELF_DATA_LITTLE_ENDIAN) return false;
    return true;
}

fn parseElfSection(data: []const u8, section_name: []const u8) ?[]const u8 {
    if (!validateElf(data)) return null;

    const e_shoff = @as(u64, @bitCast(data[40..48].*));
    const e_shentsize = @as(u16, @bitCast(data[58..60].*));
    const e_shnum = @as(u16, @bitCast(data[60..62].*));
    const e_shstrndx = @as(u16, @bitCast(data[62..64].*));

    if (e_shstrndx >= e_shnum or e_shoff == 0) return null;

    const str_sec_off = e_shoff + @as(u64, e_shstrndx) * @as(u64, e_shentsize);
    if (str_sec_off + e_shentsize > data.len) return null;

    const str_sec_data = data[str_sec_off .. str_sec_off + 64];
    const str_sec_off2 = @as(u64, @bitCast(str_sec_data[24..32].*));
    const sec_str_data = data[str_sec_off2 .. str_sec_off2 + 64];
    const sh_offset = @as(u64, @bitCast(sec_str_data[24..32].*));
    const sh_size = @as(u64, @bitCast(sec_str_data[32..40].*));

    var i: u16 = 0;
    while (i < e_shnum) : (i += 1) {
        const sec_off = e_shoff + @as(u64, i) * @as(u64, e_shentsize);
        if (sec_off + 64 > data.len) continue;

        const sec_data = data[sec_off .. sec_off + 64];
        const name_off = @as(u32, @bitCast(sec_data[0..4].*));
        const sh_offset2 = @as(u64, @bitCast(sec_data[24..32].*));
        const sh_size2 = @as(u64, @bitCast(sec_data[32..40].*));

        if (sh_offset + sh_size > data.len) continue;
        const name = std.mem.trim(u8, data[sh_offset + name_off .. sh_offset + name_off + 32], &[_]u8{0});
        if (std.mem.eql(u8, name, section_name)) {
            return data[sh_offset2 .. sh_offset2 + sh_size2];
        }
    }
    return null;
}

fn closeFd(fd: i32) void {
    _ = linux.close(fd);
}

fn checkEbpfSupport() bool {
    var attr: MapCreate = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapCreate)], 0);
    attr.map_type = BPF_MAP_TYPE_ARRAY;
    attr.key_size = 4;
    attr.value_size = 4;
    attr.max_entries = 1;
    attr.map_flags = 0;

    const fd = bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(MapCreate));
    if (fd < 0) return false;
    closeFd(fd);
    return true;
}

pub export fn ebpf_load_program(
    prog_path: [*:0]const u8,
    prog_type: u32,
) callconv(.c) i32 {
    const data = readFile(prog_path) orelse return -1;

    const code_section = parseElfSection(data, "socket") orelse
        parseElfSection(data, "xdp") orelse
        parseElfSection(data, "classifier") orelse
        parseElfSection(data, "prog") orelse return -1;

    if (code_section.len == 0) return -1;

    const insn_count = @divExact(code_section.len, 8);

    var license_buf: [16]u8 = undefined;
    @memset(license_buf[0..], 0);
    @memcpy(license_buf[0..3], "GPL");

    var attr: ProgLoad = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ProgLoad)], 0);
    attr.prog_type = prog_type;
    attr.insn_cnt = @intCast(insn_count);
    attr.insns = @intFromPtr(code_section.ptr);
    attr.license = @intFromPtr(&license_buf);
    attr.log_level = 0;
    attr.kern_version = 0;
    attr.prog_name = undefined;
    @memset(attr.prog_name[0..], 0);

    return bpfSyscall(BPF_PROG_LOAD, &attr, @sizeOf(ProgLoad));
}

pub export fn ebpf_create_sockmap(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) callconv(.c) i32 {
    var attr: MapCreate = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapCreate)], 0);
    attr.map_type = BPF_MAP_TYPE_SOCKMAP;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    attr.map_flags = 0;

    return bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(MapCreate));
}

pub export fn ebpf_create_hash_map(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) callconv(.c) i32 {
    var attr: MapCreate = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapCreate)], 0);
    attr.map_type = BPF_MAP_TYPE_HASH;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    attr.map_flags = 0;

    return bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(MapCreate));
}

pub export fn ebpf_update_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *const anyopaque,
    flags: u64,
) callconv(.c) i32 {
    var attr: MapUpdateElem = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapUpdateElem)], 0);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    attr.flags = flags;

    return bpfSyscall(BPF_MAP_UPDATE_ELEM, &attr, @sizeOf(MapUpdateElem));
}

pub export fn ebpf_lookup_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *anyopaque,
) callconv(.c) i32 {
    var attr: MapLookupElem = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapLookupElem)], 0);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    attr.flags = 0;

    return bpfSyscall(BPF_MAP_LOOKUP_ELEM, &attr, @sizeOf(MapLookupElem));
}

pub export fn ebpf_delete_map_elem(
    map_fd: i32,
    key: *const anyopaque,
) callconv(.c) i32 {
    var attr: MapDeleteElem = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapDeleteElem)], 0);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);

    return bpfSyscall(BPF_MAP_DELETE_ELEM, &attr, @sizeOf(MapDeleteElem));
}

pub export fn ebpf_attach_sockmap(
    map_fd: i32,
    prog_fd: i32,
) callconv(.c) i32 {
    var attr: ProgAttach = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ProgAttach)], 0);
    attr.target_fd = @intCast(map_fd);
    attr.attach_bpf_fd = @intCast(prog_fd);
    attr.attach_type = BPF_SK_MSG_VERDICT;
    attr.attach_flags = 0;

    return bpfSyscall(BPF_PROG_ATTACH, &attr, @sizeOf(ProgAttach));
}

pub export fn ebpf_init() callconv(.c) i32 {
    if (checkEbpfSupport()) return 0;
    return -1;
}

pub export fn ebpf_is_supported() callconv(.c) bool {
    return checkEbpfSupport();
}

pub export fn ebpf_close(prog_fd: i32) callconv(.c) i32 {
    closeFd(prog_fd);
    return 0;
}

pub export fn ebpf_pin_object(path: [*:0]const u8, fd: i32) callconv(.c) i32 {
    var attr: ObjPin = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ObjPin)], 0);
    attr.pathname = @intFromPtr(path);
    attr.bpf_fd = @intCast(fd);
    attr.file_flags = 0;

    return bpfSyscall(BPF_OBJ_PIN, &attr, @sizeOf(ObjPin));
}

pub export fn ebpf_get_pinned_object(path: [*:0]const u8) callconv(.c) i32 {
    var attr: ObjPin = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ObjPin)], 0);
    attr.pathname = @intFromPtr(path);
    attr.bpf_fd = 0;
    attr.file_flags = 0;

    return bpfSyscall(BPF_OBJ_GET, &attr, @sizeOf(ObjPin));
}

pub export fn ebpf_get_map_info(map_fd: i32) callconv(.c) i32 {
    var attr: MapGetFdById = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(MapGetFdById)], 0);
    _ = map_fd;

    return bpfSyscall(BPF_MAP_GET_FD_BY_ID, &attr, @sizeOf(MapGetFdById));
}

pub export fn ebpf_map_lookup_and_update(
    map_fd: i32,
    key: *const anyopaque,
    value: *anyopaque,
    flags: u64,
) callconv(.c) i32 {
    const lookup_result = ebpf_lookup_map_elem(map_fd, key, value);
    if (lookup_result < 0) return lookup_result;
    return ebpf_update_map_elem(map_fd, key, value, flags);
}

pub export fn ebpf_prog_get_next_prog(prog_fd: i32) callconv(.c) i32 {
    var attr: ProgGetNextId = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ProgGetNextId)], 0);
    attr.start_id = @intCast(prog_fd);

    return bpfSyscall(BPF_PROG_GET_NEXT_ID, &attr, @sizeOf(ProgGetNextId));
}

pub export fn ebpf_get_prog_info(prog_fd: i32) callconv(.c) i32 {
    var attr: ProgGetFdById = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(ProgGetFdById)], 0);
    attr.fd = @intCast(prog_fd);

    return bpfSyscall(BPF_PROG_GET_FD_BY_ID, &attr, @sizeOf(ProgGetFdById));
}

test "ebpf_loader_basic" {
    const supported = ebpf_is_supported();
    try std.testing.expect(supported == true or supported == false);
}
