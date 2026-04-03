const std = @import("std");
const ebpf_loader = @import("ebpf/loader.zig");
const cold_page_detector = @import("ebpf/cold_page_detector.zig");
const ipc_router = @import("ebpf/ipc_router.zig");
const cgroup_manager = @import("cgroups/manager.zig");
const iouring_wrapper = @import("iouring/wrapper.zig");
const criu_hibernation = @import("hibernation/criu.zig");
const cpu_pinning = @import("cpu_pinning.zig");
const vector_ops = @import("preprocess/vector.zig");
const bloom_filter = @import("preprocess/bloom.zig");
const linux_zig_core = @import("src/linux_zig_core.zig");

pub const version = "0.2.0";

const OK_MESSAGE: [*:0]const u8 = "ok";

pub const InitResult = extern struct {
    success: bool,
    error_message: [*:0]const u8,
};

pub export fn liblinux_zig_init() InitResult {
    return InitResult{
        .success = true,
        .error_message = OK_MESSAGE,
    };
}

pub export fn liblinux_zig_get_version() [*:0]const u8 {
    return "0.2.0";
}

pub export fn liblinux_zig_get_component_count() u32 {
    return 11;
}

pub export fn liblinux_zig_ebpf_load_program(
    prog_path: [*:0]const u8,
    prog_type: u32,
) i32 {
    return ebpf_loader.ebpf_load_program(prog_path, prog_type);
}

pub export fn liblinux_zig_ebpf_create_sockmap(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    return ebpf_loader.ebpf_create_sockmap(key_size, value_size, max_entries);
}

pub export fn liblinux_zig_ebpf_create_hash_map(
    key_size: u32,
    value_size: u32,
    max_entries: u32,
) i32 {
    return ebpf_loader.ebpf_create_hash_map(key_size, value_size, max_entries);
}

pub export fn liblinux_zig_ebpf_update_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *const anyopaque,
    flags: u64,
) i32 {
    return ebpf_loader.ebpf_update_map_elem(map_fd, key, value, flags);
}

pub export fn liblinux_zig_ebpf_lookup_map_elem(
    map_fd: i32,
    key: *const anyopaque,
    value: *anyopaque,
) i32 {
    return ebpf_loader.ebpf_lookup_map_elem(map_fd, key, value);
}

pub export fn liblinux_zig_ebpf_delete_map_elem(
    map_fd: i32,
    key: *const anyopaque,
) i32 {
    return ebpf_loader.ebpf_delete_map_elem(map_fd, key);
}

pub export fn liblinux_zig_ebpf_is_supported() bool {
    return ebpf_loader.ebpf_is_supported();
}

pub export fn liblinux_zig_cgroup_create(name: [*:0]const u8) i32 {
    return cgroup_manager.cgroup_create(name);
}

pub export fn liblinux_zig_cgroup_destroy(name: [*:0]const u8) i32 {
    return cgroup_manager.cgroup_destroy(name);
}

pub export fn liblinux_zig_cgroup_freeze(name: [*:0]const u8) i32 {
    return cgroup_manager.cgroup_freeze(name);
}

pub export fn liblinux_zig_cgroup_thaw(name: [*:0]const u8) i32 {
    return cgroup_manager.cgroup_thaw(name);
}

pub export fn liblinux_zig_cgroup_set_cpu_limit(name: [*:0]const u8, quota: i64, period: u64) i32 {
    return cgroup_manager.cgroup_set_cpu_limit(name, quota, period);
}

pub export fn liblinux_zig_cgroup_set_memory_limit(name: [*:0]const u8, limit_bytes: u64) i32 {
    return cgroup_manager.cgroup_set_memory_limit(name, limit_bytes);
}

pub export fn liblinux_zig_cgroup_set_io_limit(
    name: [*:0]const u8,
    dev_major: u32,
    dev_minor: u32,
    rbps: u64,
    wbps: u64,
) i32 {
    return cgroup_manager.cgroup_set_io_limit(name, dev_major, dev_minor, rbps, wbps);
}

pub export fn liblinux_zig_cgroup_add_process(name: [*:0]const u8, pid: i32) i32 {
    return cgroup_manager.cgroup_add_process(name, pid);
}

pub export fn liblinux_zig_pin_thread_to_core(pid: i32, core_mask: u64) i32 {
    return cpu_pinning.pin_thread_to_core(pid, core_mask);
}

pub export fn liblinux_zig_pin_current_thread(core: u32) i32 {
    return cpu_pinning.pin_current_thread(core);
}

pub export fn liblinux_zig_get_thread_affinity(pid: i32, core_mask: *u64) i32 {
    return cpu_pinning.get_thread_affinity(pid, core_mask);
}

pub export fn liblinux_zig_unpin_thread(pid: i32) i32 {
    return cpu_pinning.unpin_thread(pid);
}

pub export fn liblinux_zig_get_cpu_count() i32 {
    return cpu_pinning.get_cpu_count();
}

pub export fn liblinux_zig_criu_checkpoint(pid: i32, images_dir: [*:0]const u8) i32 {
    return criu_hibernation.criu_checkpoint(pid, images_dir);
}

pub export fn liblinux_zig_criu_restore(images_dir: [*:0]const u8) i32 {
    return criu_hibernation.criu_restore(images_dir);
}

pub export fn liblinux_zig_criu_pre_dump(pid: i32, images_dir: [*:0]const u8) i32 {
    return criu_hibernation.criu_pre_dump(pid, images_dir);
}

pub export fn liblinux_zig_criu_check() i32 {
    return criu_hibernation.criu_check();
}

pub export fn liblinux_zig_bloom_create(
    expected_items: u32,
    false_positive_rate: f64,
) ?*bloom_filter.BloomFilter {
    return bloom_filter.bloom_create(expected_items, false_positive_rate);
}

pub export fn liblinux_zig_bloom_add(
    filter: *bloom_filter.BloomFilter,
    item: [*]const u8,
    len: usize,
) void {
    bloom_filter.bloom_add(filter, item, len);
}

pub export fn liblinux_zig_bloom_might_contain(
    filter: *bloom_filter.BloomFilter,
    item: [*]const u8,
    len: usize,
) bool {
    return bloom_filter.bloom_might_contain(filter, item, len);
}

pub export fn liblinux_zig_bloom_destroy(filter: *bloom_filter.BloomFilter) void {
    bloom_filter.bloom_destroy(filter);
}

pub export fn liblinux_zig_bloom_clear(filter: *bloom_filter.BloomFilter) void {
    bloom_filter.bloom_clear(filter);
}

pub export fn liblinux_zig_iouring_init(
    ring: *iouring_wrapper.IoUring,
    entries: u32,
    flags: u32,
) i32 {
    return iouring_wrapper.iouring_init(ring, entries, flags);
}

pub export fn liblinux_zig_iouring_submit_read(
    ring_fd: i32,
    fd: i32,
    buf: [*]u8,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    return iouring_wrapper.iouring_submit_read(ring_fd, fd, buf, len, offset, user_data);
}

pub export fn liblinux_zig_iouring_submit_write(
    ring_fd: i32,
    fd: i32,
    buf: [*]const u8,
    len: usize,
    offset: u64,
    user_data: u64,
) i32 {
    return iouring_wrapper.iouring_submit_write(ring_fd, fd, buf, len, offset, user_data);
}

pub export fn liblinux_zig_iouring_register_buffers(
    ring_fd: i32,
    buffers: *anyopaque,
    nr_buffers: u32,
) i32 {
    return iouring_wrapper.iouring_register_buffers(ring_fd, buffers, nr_buffers);
}

pub export fn liblinux_zig_iouring_close(ring_fd: i32) i32 {
    return iouring_wrapper.iouring_close(ring_fd);
}

pub export fn liblinux_zig_init_router(prog_path: [*:0]const u8) i32 {
    return ipc_router.init_router(prog_path);
}

pub export fn liblinux_zig_create_sockmap(key_size: u32, value_size: u32, max_entries: u32) i32 {
    return ipc_router.create_sockmap(key_size, value_size, max_entries);
}

pub export fn liblinux_zig_create_route_map(key_size: u32, value_size: u32, max_entries: u32) i32 {
    return ipc_router.create_route_map(key_size, value_size, max_entries);
}

pub export fn liblinux_zig_add_route(map_fd: i32, src: u32, dst: u32, weight: u8, urgency: u8, ring_fd: i32) i32 {
    return ipc_router.add_route(map_fd, src, dst, weight, urgency, ring_fd);
}

pub export fn liblinux_zig_remove_route(map_fd: i32, src: u32) i32 {
    return ipc_router.remove_route(map_fd, src);
}

pub export fn liblinux_zig_should_forward(token_urgency: u8, route_weight: u8, signal_type: u8) bool {
    return ipc_router.should_forward(token_urgency, route_weight, signal_type);
}

pub export fn liblinux_zig_get_route_priority(src_peer: u32, signal_type: u8, urgency: u8) u8 {
    return ipc_router.get_route_priority(src_peer, signal_type, urgency);
}

pub export fn liblinux_zig_calculate_weight(urgency: u8, queue_depth: u32, bandwidth_available: u32) u8 {
    return ipc_router.calculate_weight(urgency, queue_depth, bandwidth_available);
}

pub export fn liblinux_zig_f32_dot_product_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) f32 {
    return vector_ops.f32_dot_product_simd(a, b, len);
}

pub export fn liblinux_zig_f32_normalize_simd(
    a: [*]f32,
    len: usize,
) f32 {
    return vector_ops.f32_normalize_simd(a, len);
}

pub export fn liblinux_zig_f32_scale_simd(
    a: [*]f32,
    scale: f32,
    len: usize,
) void {
    vector_ops.f32_scale_simd(a, scale, len);
}

pub export fn liblinux_zig_f32_add_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) void {
    vector_ops.f32_add_simd(a, b, len);
}

pub export fn liblinux_zig_f32_sub_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) void {
    vector_ops.f32_sub_simd(a, b, len);
}

pub export fn liblinux_zig_f32_l2_distance_sq_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) f32 {
    return vector_ops.f32_l2_distance_sq_simd(a, b, len);
}

pub export fn liblinux_zig_f64_dot_product_simd(
    a: [*]f64,
    b: [*]f64,
    len: usize,
) f64 {
    return vector_ops.f64_dot_product_simd(a, b, len);
}

pub export fn liblinux_zig_f64_normalize_simd(
    a: [*]f64,
    len: usize,
) f64 {
    return vector_ops.f64_normalize_simd(a, len);
}

pub export fn liblinux_zig_f64_scale_simd(
    a: [*]f64,
    scale: f64,
    len: usize,
) void {
    vector_ops.f64_scale_simd(a, scale, len);
}

pub export fn liblinux_zig_i32_sum_simd(
    a: [*]i32,
    len: usize,
) i32 {
    return vector_ops.i32_sum_simd(a, len);
}

pub export fn liblinux_zig_i32_max_simd(
    a: [*]i32,
    len: usize,
) i32 {
    return vector_ops.i32_max_simd(a, len);
}

pub export fn liblinux_zig_i32_min_simd(
    a: [*]i32,
    len: usize,
) i32 {
    return vector_ops.i32_min_simd(a, len);
}

pub export fn liblinux_zig_freeze_cgroup(path: [*:0]const u8) i32 {
    return linux_zig_core.freeze_cgroup(path);
}

pub export fn liblinux_zig_thaw_cgroup(path: [*:0]const u8) i32 {
    return linux_zig_core.thaw_cgroup(path);
}

pub export fn liblinux_zig_load_ebpf_program(path: [*:0]const u8) i32 {
    return linux_zig_core.load_ebpf_program(path);
}

pub export fn liblinux_zig_compress_and_store(
    pid: u32,
    addr: u64,
    len: usize,
    path: [*:0]const u8,
) i32 {
    return linux_zig_core.zig_compress_and_store(pid, addr, len, path);
}

test "liblinux_zig_init" {
    const result = liblinux_zig_init();
    try std.testing.expect(result.success);
}
