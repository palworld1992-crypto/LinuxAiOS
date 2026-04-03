#ifndef ZIG_BINDINGS_H
#define ZIG_BINDINGS_H

#include <stdint.h>

int32_t zig_ebpf_load(const char* program_path);
int32_t zig_cgroup_freeze(const char* path);
int32_t zig_cgroup_thaw(const char* path);

#endif