const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Main library: linux_zig_core
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("src/linux_zig_core.zig"),
        .target = target,
        .optimize = optimize,
    });
    lib_mod.pic = true;

    const lib = b.addLibrary(.{
        .name = "linux_zig",
        .root_module = lib_mod,
        .linkage = .static,
    });
    lib.linkage = .static;
    b.installArtifact(lib);

    // eBPF loader
    const ebpf_loader_mod = b.createModule(.{
        .root_source_file = b.path("ebpf/loader.zig"),
        .target = target,
        .optimize = optimize,
    });
    ebpf_loader_mod.pic = true;

    const ebpf_loader_lib = b.addLibrary(.{
        .name = "ebpf_loader",
        .root_module = ebpf_loader_mod,
        .linkage = .static,
    });
    b.installArtifact(ebpf_loader_lib);

    // eBPF cold page detector
    const cold_page_mod = b.createModule(.{
        .root_source_file = b.path("ebpf/cold_page_detector.zig"),
        .target = target,
        .optimize = optimize,
    });
    cold_page_mod.pic = true;

    const cold_page_lib = b.addLibrary(.{
        .name = "ebpf_coldpage",
        .root_module = cold_page_mod,
        .linkage = .static,
    });
    b.installArtifact(cold_page_lib);

    // eBPF IPC router
    const ipc_router_mod = b.createModule(.{
        .root_source_file = b.path("ebpf/ipc_router.zig"),
        .target = target,
        .optimize = optimize,
    });
    ipc_router_mod.pic = true;

    const ipc_router_lib = b.addLibrary(.{
        .name = "ebpf_ipc_router",
        .root_module = ipc_router_mod,
        .linkage = .static,
    });
    b.installArtifact(ipc_router_lib);

    // Cgroups manager
    const cgroup_mod = b.createModule(.{
        .root_source_file = b.path("cgroups/manager.zig"),
        .target = target,
        .optimize = optimize,
    });
    cgroup_mod.pic = true;

    const cgroup_lib = b.addLibrary(.{
        .name = "cgroup_manager",
        .root_module = cgroup_mod,
        .linkage = .static,
    });
    b.installArtifact(cgroup_lib);

    // io_uring wrapper
    const iouring_mod = b.createModule(.{
        .root_source_file = b.path("iouring/wrapper.zig"),
        .target = target,
        .optimize = optimize,
    });
    iouring_mod.pic = true;

    const iouring_lib = b.addLibrary(.{
        .name = "iouring_wrapper",
        .root_module = iouring_mod,
        .linkage = .static,
    });
    b.installArtifact(iouring_lib);

    // CRIU hibernation
    const criu_mod = b.createModule(.{
        .root_source_file = b.path("hibernation/criu.zig"),
        .target = target,
        .optimize = optimize,
    });
    criu_mod.pic = true;

    const criu_lib = b.addLibrary(.{
        .name = "criu_hibernation",
        .root_module = criu_mod,
        .linkage = .static,
    });
    b.installArtifact(criu_lib);

    // CPU pinning
    const pinning_mod = b.createModule(.{
        .root_source_file = b.path("cpu_pinning.zig"),
        .target = target,
        .optimize = optimize,
    });
    pinning_mod.pic = true;

    const pinning_lib = b.addLibrary(.{
        .name = "cpu_pinning",
        .root_module = pinning_mod,
        .linkage = .static,
    });
    b.installArtifact(pinning_lib);

    // Vector SIMD operations
    const vector_mod = b.createModule(.{
        .root_source_file = b.path("preprocess/vector.zig"),
        .target = target,
        .optimize = optimize,
    });
    vector_mod.pic = true;

    const vector_lib = b.addLibrary(.{
        .name = "preprocess_vector",
        .root_module = vector_mod,
        .linkage = .static,
    });
    b.installArtifact(vector_lib);

    // Bloom filter
    const bloom_mod = b.createModule(.{
        .root_source_file = b.path("preprocess/bloom.zig"),
        .target = target,
        .optimize = optimize,
    });
    bloom_mod.pic = true;

    const bloom_lib = b.addLibrary(.{
        .name = "preprocess_bloom",
        .root_module = bloom_mod,
        .linkage = .static,
    });
    b.installArtifact(bloom_lib);

    // Unified library: liblinux_zig
    const unified_mod = b.createModule(.{
        .root_source_file = b.path("liblinux_zig.zig"),
        .target = target,
        .optimize = optimize,
    });
    unified_mod.pic = true;

    const unified_lib = b.addLibrary(.{
        .name = "liblinux_zig",
        .root_module = unified_mod,
        .linkage = .static,
    });
    b.installArtifact(unified_lib);
}
