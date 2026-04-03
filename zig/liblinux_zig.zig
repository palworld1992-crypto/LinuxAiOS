const std = @import("std");
const linux_zig_core = @import("src/linux_zig_core.zig");
const ebpf_loader = @import("ebpf/loader.zig");
const cold_page_detector = @import("ebpf/cold_page_detector.zig");
const ipc_router = @import("ebpf/ipc_router.zig");
const cgroup_manager = @import("cgroups/manager.zig");
const iouring_wrapper = @import("iouring/wrapper.zig");
const criu_hibernation = @import("hibernation/criu.zig");
const cpu_pinning = @import("cpu_pinning.zig");
const vector_ops = @import("preprocess/vector.zig");
const bloom_filter = @import("preprocess/bloom.zig");

pub const version = "0.1.0";

const OK_MESSAGE: [*:0]const u8 = "ok";

pub const InitResult = extern struct {
    success: bool,
    error_message: [*:0]const u8,
};

pub export fn liblinux_zig_init() InitResult {
    _ = linux_zig_core;
    _ = ebpf_loader;
    _ = cold_page_detector;
    _ = ipc_router;
    _ = cgroup_manager;
    _ = iouring_wrapper;
    _ = criu_hibernation;
    _ = cpu_pinning;
    _ = vector_ops;
    _ = bloom_filter;

    return InitResult{
        .success = true,
        .error_message = OK_MESSAGE,
    };
}

pub export fn liblinux_zig_get_version() [*:0]const u8 {
    return "0.1.0";
}

pub export fn liblinux_zig_get_component_count() u32 {
    return 11;
}

test "liblinux_zig_init" {
    const result = liblinux_zig_init();
    try std.testing.expect(result.success);
}
