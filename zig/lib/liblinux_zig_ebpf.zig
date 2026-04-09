const std = @import("std");
const linux = std.os.linux;
const types = @import("liblinux_zig_types.zig");

const BPF_MAP_CREATE: u32 = 0;
const BPF_MAP_UPDATE_ELEM: u32 = 2;
const BPF_MAP_LOOKUP_ELEM: u32 = 1;
const BPF_MAP_DELETE_ELEM: u32 = 3;
const BPF_MAP_TYPE_HASH: u32 = 1;
const BPF_MAP_TYPE_SOCKMAP: u32 = 9;

fn bpfSyscall(cmd: u32, attr: *const anyopaque, size: usize) i32 {
    const result = linux.syscall3(.bpf, cmd, @intFromPtr(attr), size);
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

pub export fn liblinux_zig_ebpf_load_program(prog_path: [*:0]const u8, prog_type: u32) callconv(.c) i32 {
    return ebpf_load_elf(prog_path, prog_type);
}

fn ebpf_load_elf(prog_path: [*:0]const u8, prog_type: u32) i32 {
    const raw_fd = linux.open(prog_path, linux.O{ .ACCMODE = .RDONLY, .CLOEXEC = true }, 0);
    const fd: i32 = @intCast(raw_fd);
    if (fd < 0) return -1;
    defer {
        _ = linux.close(fd);
    }

    var header: [256]u8 = undefined;
    const n = linux.read(fd, &header, header.len);
    if (n < 64) return -1;

    const magic = @as(u32, @bitCast(header[0..4].*));
    if (magic != 0x464C457F) return -1;
    if (header[4] != 2) return -1;
    if (header[5] != 1) return -1;

    const e_shoff = @as(u64, @bitCast(header[40..48].*));
    const e_shentsize = @as(u16, @bitCast(header[58..60].*));
    const e_shnum = @as(u16, @bitCast(header[60..62].*));
    const e_shstrndx = @as(u16, @bitCast(header[62..64].*));

    if (e_shstrndx >= e_shnum or e_shoff == 0) return -1;

    const str_sec_off = e_shoff + @as(u64, e_shstrndx) * @as(u64, e_shentsize);
    if (str_sec_off + e_shentsize > header.len) return -1;

    const str_sec_data = header[str_sec_off..];
    const str_sec_off2 = @as(u64, @bitCast(str_sec_data[24..32].*));

    var code_ptr: ?[]const u8 = null;
    var code_size: usize = 0;

    var i: u16 = 0;
    while (i < e_shnum) : (i += 1) {
        const sec_off = e_shoff + @as(u64, i) * @as(u64, e_shentsize);
        if (sec_off + 64 > header.len) continue;

        const sec_data = header[sec_off..];
        const name_off = @as(u32, @bitCast(sec_data[0..4].*));
        const sh_offset = @as(u64, @bitCast(sec_data[24..32].*));
        const sh_size = @as(u64, @bitCast(sec_data[32..40].*));
        const sh_type = @as(u32, @bitCast(sec_data[4..8].*));

        if (sh_type != 1) continue;
        if (str_sec_off2 + name_off + 32 > header.len) continue;

        const name_start = str_sec_off2 + name_off;
        const name_end = name_start + 32;
        if (name_end > header.len) continue;

        const name = std.mem.trim(u8, header[name_start..name_end], &[_]u8{0});
        if (std.mem.eql(u8, name, "socket") or
            std.mem.eql(u8, name, "xdp") or
            std.mem.eql(u8, name, "classifier") or
            std.mem.eql(u8, name, "prog"))
        {
            code_size = @intCast(sh_size);
            if (code_size > 0) {
                const data_off = @as(usize, @intCast(sh_offset));
                if (data_off < 4096) {
                    code_ptr = header[data_off .. data_off + code_size];
                }
            }
            break;
        }
    }

    if (code_ptr == null or code_size == 0) return -1;
    const code = code_ptr.?;

    const insn_count = @divExact(code_size, 8);
    if (insn_count == 0) return -1;

    var license_buf: [16]u8 = undefined;
    @memset(license_buf[0..], 0);
    @memcpy(license_buf[0..3], "GPL");

    var attr: types.BpfProgLoadAttr = undefined;
    @memset(@as([*]u8, @ptrCast(&attr))[0..@sizeOf(types.BpfProgLoadAttr)], 0);
    attr.prog_type = prog_type;
    attr.insn_cnt = @intCast(insn_count);
    attr.insns = @intFromPtr(code.ptr);
    attr.license = @intFromPtr(&license_buf);
    attr.kern_version = 0;

    return bpfSyscall2(5, &attr, @sizeOf(types.BpfProgLoadAttr));
}

fn bpfSyscall2(cmd: u32, attr: *const anyopaque, size: usize) i32 {
    const result = linux.syscall3(.bpf, cmd, @intFromPtr(attr), size);
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

pub export fn liblinux_zig_ebpf_create_sockmap(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    var attr: types.BpfMapCreateAttr = std.mem.zeroes(types.BpfMapCreateAttr);
    attr.map_type = BPF_MAP_TYPE_SOCKMAP;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    return bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(types.BpfMapCreateAttr));
}

pub export fn liblinux_zig_ebpf_create_hash_map(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    var attr: types.BpfMapCreateAttr = std.mem.zeroes(types.BpfMapCreateAttr);
    attr.map_type = BPF_MAP_TYPE_HASH;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    return bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(types.BpfMapCreateAttr));
}

pub export fn liblinux_zig_ebpf_update_map_elem(map_fd: i32, key: *const anyopaque, value: *const anyopaque, flags: u64) callconv(.c) i32 {
    var attr: types.BpfMapElemAttr = std.mem.zeroes(types.BpfMapElemAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    attr.flags = flags;
    return bpfSyscall(BPF_MAP_UPDATE_ELEM, &attr, @sizeOf(types.BpfMapElemAttr));
}

pub export fn liblinux_zig_ebpf_lookup_map_elem(map_fd: i32, key: *const anyopaque, value: *anyopaque) callconv(.c) i32 {
    var attr: types.BpfMapLookupAttr = std.mem.zeroes(types.BpfMapLookupAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    return bpfSyscall(BPF_MAP_LOOKUP_ELEM, &attr, @sizeOf(types.BpfMapLookupAttr));
}

pub export fn liblinux_zig_ebpf_delete_map_elem(map_fd: i32, key: *const anyopaque) callconv(.c) i32 {
    var attr: types.BpfMapDeleteAttr = std.mem.zeroes(types.BpfMapDeleteAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    return bpfSyscall(BPF_MAP_DELETE_ELEM, &attr, @sizeOf(types.BpfMapDeleteAttr));
}

pub export fn liblinux_zig_ebpf_is_supported() callconv(.c) bool {
    var attr: types.BpfMapCreateAttr = std.mem.zeroes(types.BpfMapCreateAttr);
    attr.map_type = BPF_MAP_TYPE_HASH;
    attr.key_size = 4;
    attr.value_size = 4;
    attr.max_entries = 1;
    const fd = bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(types.BpfMapCreateAttr));
    if (fd < 0) return false;
    _ = linux.close(@as(i32, @intCast(fd)));
    return true;
}
