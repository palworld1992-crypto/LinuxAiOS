const std = @import("std");

pub const version = @import("liblinux_zig_types.zig").version;
pub const InitResult = @import("liblinux_zig_types.zig").InitResult;
pub const BloomFilter = @import("liblinux_zig_types.zig").BloomFilter;
pub const IoUring = @import("liblinux_zig_types.zig").IoUring;
pub const RouteEntry = @import("liblinux_zig_types.zig").RouteEntry;
pub const RouteStats = @import("liblinux_zig_types.zig").RouteStats;
pub const ColdPageEvent = @import("liblinux_zig_types.zig").ColdPageEvent;

const OK_MESSAGE: [*:0]const u8 = "ok";

pub export fn liblinux_zig_init() callconv(.c) InitResult {
    return InitResult{
        .success = true,
        .error_message = OK_MESSAGE,
    };
}

pub export fn liblinux_zig_get_version() [*:0]const u8 {
    return "0.2.0";
}

pub export fn liblinux_zig_get_component_count() callconv(.c) u32 {
    return 11;
}

pub const liblinux_zig_pin_thread_to_core = @import("liblinux_zig_cpu.zig").liblinux_zig_pin_thread_to_core;
pub const liblinux_zig_pin_current_thread = @import("liblinux_zig_cpu.zig").liblinux_zig_pin_current_thread;
pub const liblinux_zig_get_thread_affinity = @import("liblinux_zig_cpu.zig").liblinux_zig_get_thread_affinity;
pub const liblinux_zig_unpin_thread = @import("liblinux_zig_cpu.zig").liblinux_zig_unpin_thread;
pub const liblinux_zig_get_cpu_count = @import("liblinux_zig_cpu.zig").liblinux_zig_get_cpu_count;

pub const liblinux_zig_cgroup_create = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_create;
pub const liblinux_zig_cgroup_destroy = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_destroy;
pub const liblinux_zig_cgroup_freeze = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_freeze;
pub const liblinux_zig_cgroup_thaw = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_thaw;
pub const liblinux_zig_cgroup_set_cpu_limit = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_set_cpu_limit;
pub const liblinux_zig_cgroup_set_memory_limit = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_set_memory_limit;
pub const liblinux_zig_cgroup_set_io_limit = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_set_io_limit;
pub const liblinux_zig_cgroup_add_process = @import("liblinux_zig_cgroup.zig").liblinux_zig_cgroup_add_process;

pub const liblinux_zig_ebpf_load_program = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_load_program;
pub const liblinux_zig_ebpf_create_sockmap = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_create_sockmap;
pub const liblinux_zig_ebpf_create_hash_map = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_create_hash_map;
pub const liblinux_zig_ebpf_update_map_elem = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_update_map_elem;
pub const liblinux_zig_ebpf_lookup_map_elem = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_lookup_map_elem;
pub const liblinux_zig_ebpf_delete_map_elem = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_delete_map_elem;
pub const liblinux_zig_ebpf_is_supported = @import("liblinux_zig_ebpf.zig").liblinux_zig_ebpf_is_supported;

pub const liblinux_zig_criu_checkpoint = @import("liblinux_zig_criu.zig").liblinux_zig_criu_checkpoint;
pub const liblinux_zig_criu_restore = @import("liblinux_zig_criu.zig").liblinux_zig_criu_restore;
pub const liblinux_zig_criu_pre_dump = @import("liblinux_zig_criu.zig").liblinux_zig_criu_pre_dump;
pub const liblinux_zig_criu_check = @import("liblinux_zig_criu.zig").liblinux_zig_criu_check;

pub const liblinux_zig_bloom_create = @import("liblinux_zig_bloom.zig").liblinux_zig_bloom_create;
pub const liblinux_zig_bloom_add = @import("liblinux_zig_bloom.zig").liblinux_zig_bloom_add;
pub const liblinux_zig_bloom_might_contain = @import("liblinux_zig_bloom.zig").liblinux_zig_bloom_might_contain;
pub const liblinux_zig_bloom_destroy = @import("liblinux_zig_bloom.zig").liblinux_zig_bloom_destroy;
pub const liblinux_zig_bloom_clear = @import("liblinux_zig_bloom.zig").liblinux_zig_bloom_clear;

pub const liblinux_zig_iouring_init = @import("liblinux_zig_iouring.zig").liblinux_zig_iouring_init;
pub const liblinux_zig_iouring_submit_read = @import("liblinux_zig_iouring.zig").liblinux_zig_iouring_submit_read;
pub const liblinux_zig_iouring_submit_write = @import("liblinux_zig_iouring.zig").liblinux_zig_iouring_submit_write;
pub const liblinux_zig_iouring_register_buffers = @import("liblinux_zig_iouring.zig").liblinux_zig_iouring_register_buffers;
pub const liblinux_zig_iouring_close = @import("liblinux_zig_iouring.zig").liblinux_zig_iouring_close;

pub const liblinux_zig_init_router = @import("liblinux_zig_router.zig").liblinux_zig_init_router;
pub const liblinux_zig_create_sockmap = @import("liblinux_zig_router.zig").liblinux_zig_create_sockmap;
pub const liblinux_zig_create_route_map = @import("liblinux_zig_router.zig").liblinux_zig_create_route_map;
pub const liblinux_zig_add_route = @import("liblinux_zig_router.zig").liblinux_zig_add_route;
pub const liblinux_zig_remove_route = @import("liblinux_zig_router.zig").liblinux_zig_remove_route;
pub const liblinux_zig_should_forward = @import("liblinux_zig_router.zig").liblinux_zig_should_forward;
pub const liblinux_zig_get_route_priority = @import("liblinux_zig_router.zig").liblinux_zig_get_route_priority;
pub const liblinux_zig_calculate_weight = @import("liblinux_zig_router.zig").liblinux_zig_calculate_weight;

pub const liblinux_zig_f32_dot_product_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_dot_product_simd;
pub const liblinux_zig_f32_normalize_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_normalize_simd;
pub const liblinux_zig_f32_scale_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_scale_simd;
pub const liblinux_zig_f32_add_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_add_simd;
pub const liblinux_zig_f32_sub_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_sub_simd;
pub const liblinux_zig_f32_l2_distance_sq_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f32_l2_distance_sq_simd;
pub const liblinux_zig_f64_dot_product_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f64_dot_product_simd;
pub const liblinux_zig_f64_normalize_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f64_normalize_simd;
pub const liblinux_zig_f64_scale_simd = @import("liblinux_zig_vector.zig").liblinux_zig_f64_scale_simd;
pub const liblinux_zig_i32_sum_simd = @import("liblinux_zig_vector.zig").liblinux_zig_i32_sum_simd;
pub const liblinux_zig_i32_max_simd = @import("liblinux_zig_vector.zig").liblinux_zig_i32_max_simd;
pub const liblinux_zig_i32_min_simd = @import("liblinux_zig_vector.zig").liblinux_zig_i32_min_simd;

pub const liblinux_zig_compress_and_store = @import("liblinux_zig_compress.zig").liblinux_zig_compress_and_store;

pub export fn liblinux_zig_freeze_cgroup(path: [*:0]const u8) callconv(.c) i32 {
    return liblinux_zig_cgroup_freeze(path);
}

pub export fn liblinux_zig_thaw_cgroup(path: [*:0]const u8) callconv(.c) i32 {
    return liblinux_zig_cgroup_thaw(path);
}

pub export fn liblinux_zig_load_ebpf_program(path: [*:0]const u8) callconv(.c) i32 {
    return liblinux_zig_ebpf_load_program(path, 0);
}

test "liblinux_zig_init" {
    const result = liblinux_zig_init();
    try std.testing.expect(result.success);
}

test "liblinux_zig_cpu_count" {
    const count = liblinux_zig_get_cpu_count();
    try std.testing.expect(count > 0);
}

test "liblinux_zig_bloom" {
    const filter = liblinux_zig_bloom_create(1000, 0.01);
    try std.testing.expect(filter != null);
    const item = "hello";
    liblinux_zig_bloom_add(filter.?, item, item.len);
    try std.testing.expect(liblinux_zig_bloom_might_contain(filter.?, item, item.len));
    liblinux_zig_bloom_destroy(filter.?);
}

test "liblinux_zig_vector_simd" {
    var a = [_]f32{ 1.0, 2.0, 3.0, 4.0 };
    var b = [_]f32{ 1.0, 1.0, 1.0, 1.0 };
    const dot = liblinux_zig_f32_dot_product_simd(&a, &b, 4);
    try std.testing.expect(dot > 0.0);
}
