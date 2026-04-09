const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseFast,
    });

    const types_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_types.zig"),
        .target = target,
        .optimize = optimize,
    });
    types_mod.pic = true;

    const cpu_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_cpu.zig"),
        .target = target,
        .optimize = optimize,
    });
    cpu_mod.pic = true;

    const cgroup_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_cgroup.zig"),
        .target = target,
        .optimize = optimize,
    });
    cgroup_mod.pic = true;

    const ebpf_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_ebpf.zig"),
        .target = target,
        .optimize = optimize,
    });
    ebpf_mod.pic = true;
    ebpf_mod.addImport("lib/liblinux_zig_types", types_mod);

    const criu_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_criu.zig"),
        .target = target,
        .optimize = optimize,
    });
    criu_mod.pic = true;

    const bloom_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_bloom.zig"),
        .target = target,
        .optimize = optimize,
    });
    bloom_mod.pic = true;
    bloom_mod.addImport("lib/liblinux_zig_types", types_mod);

    const iouring_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_iouring.zig"),
        .target = target,
        .optimize = optimize,
    });
    iouring_mod.pic = true;
    iouring_mod.addImport("lib/liblinux_zig_types", types_mod);

    const router_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_router.zig"),
        .target = target,
        .optimize = optimize,
    });
    router_mod.pic = true;
    router_mod.addImport("lib/liblinux_zig_types", types_mod);
    router_mod.addImport("lib/liblinux_zig_ebpf", ebpf_mod);

    const vector_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_vector.zig"),
        .target = target,
        .optimize = optimize,
    });
    vector_mod.pic = true;
    vector_mod.addImport("lib/liblinux_zig_types", types_mod);

    const compress_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig_compress.zig"),
        .target = target,
        .optimize = optimize,
    });
    compress_mod.pic = true;

    const unified_mod = b.createModule(.{
        .root_source_file = b.path("lib/liblinux_zig.zig"),
        .target = target,
        .optimize = optimize,
    });
    unified_mod.pic = true;
    unified_mod.addImport("lib/liblinux_zig_types", types_mod);
    unified_mod.addImport("lib/liblinux_zig_cpu", cpu_mod);
    unified_mod.addImport("lib/liblinux_zig_cgroup", cgroup_mod);
    unified_mod.addImport("lib/liblinux_zig_ebpf", ebpf_mod);
    unified_mod.addImport("lib/liblinux_zig_criu", criu_mod);
    unified_mod.addImport("lib/liblinux_zig_bloom", bloom_mod);
    unified_mod.addImport("lib/liblinux_zig_iouring", iouring_mod);
    unified_mod.addImport("lib/liblinux_zig_router", router_mod);
    unified_mod.addImport("lib/liblinux_zig_vector", vector_mod);
    unified_mod.addImport("lib/liblinux_zig_compress", compress_mod);

    const unified_lib = b.addLibrary(.{
        .name = "liblinux_zig",
        .root_module = unified_mod,
        .linkage = .static,
    });
    b.installArtifact(unified_lib);

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

    const cgroup_manager_mod = b.createModule(.{
        .root_source_file = b.path("cgroups/manager.zig"),
        .target = target,
        .optimize = optimize,
    });
    cgroup_manager_mod.pic = true;

    const cgroup_lib = b.addLibrary(.{
        .name = "cgroup_manager",
        .root_module = cgroup_manager_mod,
        .linkage = .static,
    });
    b.installArtifact(cgroup_lib);

    const iouring_wrapper_mod = b.createModule(.{
        .root_source_file = b.path("iouring/wrapper.zig"),
        .target = target,
        .optimize = optimize,
    });
    iouring_wrapper_mod.pic = true;

    const iouring_lib = b.addLibrary(.{
        .name = "iouring_wrapper",
        .root_module = iouring_wrapper_mod,
        .linkage = .static,
    });
    b.installArtifact(iouring_lib);

    const criu_hib_mod = b.createModule(.{
        .root_source_file = b.path("hibernation/criu.zig"),
        .target = target,
        .optimize = optimize,
    });
    criu_hib_mod.pic = true;

    const criu_lib = b.addLibrary(.{
        .name = "criu_hibernation",
        .root_module = criu_hib_mod,
        .linkage = .static,
    });
    b.installArtifact(criu_lib);

    const pinning_mod = b.createModule(.{
        .root_source_file = b.path("src/cpu_pinning.zig"),
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

    const vec_mod = b.createModule(.{
        .root_source_file = b.path("preprocess/vector.zig"),
        .target = target,
        .optimize = optimize,
    });
    vec_mod.pic = true;

    const vector_lib = b.addLibrary(.{
        .name = "preprocess_vector",
        .root_module = vec_mod,
        .linkage = .static,
    });
    b.installArtifact(vector_lib);

    const bloom_mod2 = b.createModule(.{
        .root_source_file = b.path("preprocess/bloom.zig"),
        .target = target,
        .optimize = optimize,
    });
    bloom_mod2.pic = true;

    const bloom_lib = b.addLibrary(.{
        .name = "preprocess_bloom",
        .root_module = bloom_mod2,
        .linkage = .static,
    });
    b.installArtifact(bloom_lib);

    // Test step
    const all_tests_step = b.step("test", "Run all tests");

    // types_test
    const types_test_mod = b.createModule(.{
        .root_source_file = b.path("tests/types_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    types_test_mod.addImport("zig_types", types_mod);
    const types_test = b.addTest(.{ .root_module = types_test_mod });
    all_tests_step.dependOn(&b.addRunArtifact(types_test).step);

    // cpu_test
    const cpu_test_mod = b.createModule(.{
        .root_source_file = b.path("tests/cpu_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const cpu_test = b.addTest(.{ .root_module = cpu_test_mod });
    all_tests_step.dependOn(&b.addRunArtifact(cpu_test).step);

    // bloom_test - needs preprocess/bloom.zig (not the C wrapper)
    const bloom_src_mod = b.createModule(.{
        .root_source_file = b.path("preprocess/bloom.zig"),
        .target = target,
        .optimize = optimize,
    });
    const bloom_test_mod = b.createModule(.{
        .root_source_file = b.path("tests/bloom_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    bloom_test_mod.addImport("zig_bloom", bloom_src_mod);
    const bloom_test = b.addTest(.{ .root_module = bloom_test_mod });
    all_tests_step.dependOn(&b.addRunArtifact(bloom_test).step);

    // vector_test - needs preprocess/vector.zig (not the C wrapper)
    const vector_src_mod = b.createModule(.{
        .root_source_file = b.path("preprocess/vector.zig"),
        .target = target,
        .optimize = optimize,
    });
    const vector_test_mod = b.createModule(.{
        .root_source_file = b.path("tests/vector_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    vector_test_mod.addImport("zig_vector", vector_src_mod);
    const vector_test = b.addTest(.{ .root_module = vector_test_mod });
    all_tests_step.dependOn(&b.addRunArtifact(vector_test).step);
}
