const std = @import("std");

pub const version = "0.2.0";

pub const InitResult = extern struct {
    success: bool,
    error_message: [*:0]const u8,
};

pub const BloomFilter = struct {
    bits: [131072]u8,
    num_bits: u32,
    num_hashes: u32,
    item_count: u32,
    expected_items: u32,
    false_positive_rate: f64,
};

pub const IoUring = extern struct {
    ring_fd: i32,
    ring_size: u32,
    mapped: bool,
};

pub const RouteEntry = struct {
    src_peer: u32,
    dst_sock: u32,
    weight: u8,
    urgency: u8,
    ring_buffer_fd: i32,
    active: bool,
};

pub const RouteStats = struct {
    total_packets: u64,
    high_urgency: u64,
    medium_urgency: u64,
    low_urgency: u64,
    dropped: u64,
    forwarded: u64,
    hebbian_updates: u64,
};

pub const ColdPageEvent = extern struct {
    pid: u32,
    addr: u64,
    timestamp: u64,
    access_count: u32,
};

pub const BpfMapCreateAttr = extern struct {
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

pub const BpfMapElemAttr = extern struct {
    map_fd: u32,
    key: u64,
    value: u64,
    flags: u64,
};

pub const BpfMapLookupAttr = extern struct {
    map_fd: u32,
    key: u64,
    value: u64,
    flags: u64,
};

pub const BpfMapDeleteAttr = extern struct {
    map_fd: u32,
    key: u64,
};

pub const IoUringSqe = extern struct {
    opcode: u8,
    flags: u8,
    ioprio: u16,
    fd: i32,
    off: u64,
    addr: u64,
    len: u32,
    rw_flags: i32,
    user_data: u64,
    buf_index: u16,
    personality: u16,
    splice_fd_in: i32,
};

pub const IoUringParams = extern struct {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32,
    wq_fd: u32,
    resv: [3]u32,
    sq_off: [8]u32,
    cq_off: [8]u32,
};

pub const BpfProgLoadAttr = extern struct {
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
