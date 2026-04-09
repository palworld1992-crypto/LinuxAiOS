const std = @import("std");
const linux = std.os.linux;
const types = @import("liblinux_zig_types.zig");

const BPF_MAP_CREATE: u32 = 0;
const BPF_MAP_UPDATE_ELEM: u32 = 2;
const BPF_MAP_LOOKUP_ELEM: u32 = 1;
const BPF_MAP_DELETE_ELEM: u32 = 3;
const BPF_MAP_TYPE_HASH: u32 = 1;
const BPF_MAP_TYPE_SOCKMAP: u32 = 9;
const BPF_PROG_LOAD: u32 = 5;
const BPF_PROG_TYPE_SK_MSG: u32 = 21;

var g_route_stats: types.RouteStats = types.RouteStats{
    .total_packets = 0,
    .high_urgency = 0,
    .medium_urgency = 0,
    .low_urgency = 0,
    .dropped = 0,
    .forwarded = 0,
    .hebbian_updates = 0,
};
var g_routes: [256]types.RouteEntry = undefined;
var g_route_count: u32 = 0;
var g_router_map_fd: i32 = -1;

fn bpfSyscall(cmd: u32, attr: *const anyopaque, size: usize) i32 {
    const result = linux.syscall3(.bpf, cmd, @intFromPtr(attr), size);
    const as_isize: isize = @bitCast(result);
    if (as_isize < 0) return -1;
    return @as(i32, @intCast(result));
}

fn bpfMapCreate(map_type: u32, key_size: u32, value_size: u32, max_entries: u32) i32 {
    var attr: types.BpfMapCreateAttr = std.mem.zeroes(types.BpfMapCreateAttr);
    attr.map_type = map_type;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = max_entries;
    return bpfSyscall(BPF_MAP_CREATE, &attr, @sizeOf(types.BpfMapCreateAttr));
}

fn bpfMapUpdateElem(map_fd: i32, key: *const anyopaque, value: *const anyopaque, flags: u64) i32 {
    var attr: types.BpfMapElemAttr = std.mem.zeroes(types.BpfMapElemAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    attr.flags = flags;
    return bpfSyscall(BPF_MAP_UPDATE_ELEM, &attr, @sizeOf(types.BpfMapElemAttr));
}

fn bpfMapLookupElem(map_fd: i32, key: *const anyopaque, value: *anyopaque) i32 {
    var attr: types.BpfMapLookupAttr = std.mem.zeroes(types.BpfMapLookupAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    attr.value = @intFromPtr(value);
    return bpfSyscall(BPF_MAP_LOOKUP_ELEM, &attr, @sizeOf(types.BpfMapLookupAttr));
}

fn bpfMapDeleteElem(map_fd: i32, key: *const anyopaque) i32 {
    var attr: types.BpfMapDeleteAttr = std.mem.zeroes(types.BpfMapDeleteAttr);
    attr.map_fd = @intCast(map_fd);
    attr.key = @intFromPtr(key);
    return bpfSyscall(BPF_MAP_DELETE_ELEM, &attr, @sizeOf(types.BpfMapDeleteAttr));
}

pub export fn liblinux_zig_init_router(prog_path: [*:0]const u8) callconv(.c) i32 {
    _ = linux.close(g_router_map_fd);
    const map_fd = bpfMapCreate(BPF_MAP_TYPE_HASH, 4, @sizeOf(types.RouteEntry), 256);
    if (map_fd < 0) return -1;
    g_router_map_fd = map_fd;
    _ = prog_path;
    return 0;
}

pub export fn liblinux_zig_create_sockmap(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    return bpfMapCreate(BPF_MAP_TYPE_SOCKMAP, key_size, value_size, max_entries);
}

pub export fn liblinux_zig_create_route_map(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    return bpfMapCreate(BPF_MAP_TYPE_HASH, key_size, value_size, max_entries);
}

pub export fn liblinux_zig_add_route(map_fd: i32, src: u32, dst: u32, weight: u8, urgency: u8, ring_fd: i32) callconv(.c) i32 {
    const fd = if (map_fd >= 0) map_fd else map_fd;
    if (fd < 0 and map_fd < 0) {
        if (g_route_count >= 256) return -1;
        g_routes[g_route_count] = types.RouteEntry{ .src_peer = src, .dst_sock = dst, .weight = weight, .urgency = urgency, .ring_buffer_fd = ring_fd, .active = true };
        g_route_count += 1;
        return 0;
    }

    const actual_fd = if (map_fd >= 0) map_fd else fd;
    var route = types.RouteEntry{ .src_peer = src, .dst_sock = dst, .weight = weight, .urgency = urgency, .ring_buffer_fd = ring_fd, .active = true };
    const key: u32 = src;
    return bpfMapUpdateElem(actual_fd, &key, &route, 0);
}

pub export fn liblinux_zig_remove_route(map_fd: i32, src: u32) callconv(.c) i32 {
    const actual_fd = if (map_fd >= 0) map_fd else map_fd;
    if (actual_fd < 0) {
        var i: u32 = 0;
        while (i < g_route_count) : (i += 1) {
            if (g_routes[i].src_peer == src) {
                g_routes[i].active = false;
                return 0;
            }
        }
        return -1;
    }
    const key: u32 = src;
    return bpfMapDeleteElem(actual_fd, &key);
}

pub export fn liblinux_zig_should_forward(token_urgency: u8, route_weight: u8, signal_type: u8) callconv(.c) bool {
    if (token_urgency >= 200) return true;
    if (signal_type == 1 and route_weight > 128) return true;
    return route_weight > 64;
}

pub export fn liblinux_zig_get_route_priority(src_peer: u32, signal_type: u8, urgency: u8) callconv(.c) u8 {
    _ = src_peer;
    var p: u16 = urgency;
    if (signal_type == 1) p = @min(p + 50, 255);
    return @intCast(p);
}

pub export fn liblinux_zig_calculate_weight(urgency: u8, queue_depth: u32, bandwidth_available: u32) callconv(.c) u8 {
    const base: u8 = if (urgency >= 200) 200 else if (urgency >= 100) 128 else 64;
    const qf = @min(queue_depth / 1000, 50);
    const bf = @min(bandwidth_available / 1000000, 50);
    return @min(base +% @as(u8, @intCast(qf)) +% @as(u8, @intCast(bf)), 255);
}

pub export fn liblinux_zig_get_route_stats() callconv(.c) *types.RouteStats {
    return &g_route_stats;
}

pub export fn liblinux_zig_reset_stats() callconv(.c) void {
    g_route_stats.total_packets = 0;
    g_route_stats.high_urgency = 0;
    g_route_stats.medium_urgency = 0;
    g_route_stats.low_urgency = 0;
    g_route_stats.dropped = 0;
    g_route_stats.forwarded = 0;
    g_route_stats.hebbian_updates = 0;
}

pub export fn liblinux_zig_find_best_route(map_fd: i32, src: u32, urgency: u8) callconv(.c) i32 {
    const actual_fd = if (map_fd >= 0) map_fd else g_router_map_fd;
    if (actual_fd >= 0) {
        var key: u32 = src;
        var value: types.RouteEntry = undefined;
        const result = bpfMapLookupElem(actual_fd, &key, &value);
        if (result >= 0 and value.active) {
            var best_idx: i32 = -1;
            var best_score: u32 = 0;
            var i: u32 = 0;
            while (i < g_route_count) : (i += 1) {
                if (!g_routes[i].active) continue;
                if (g_routes[i].src_peer != src) continue;
                const score = @as(u32, g_routes[i].weight) * 2 + @as(u32, g_routes[i].urgency);
                if (score > best_score) {
                    best_score = score;
                    best_idx = @intCast(i);
                }
            }
            return best_idx;
        }
    }

    var best_idx: i32 = -1;
    var best_score: u32 = 0;
    var i: u32 = 0;
    while (i < g_route_count) : (i += 1) {
        if (!g_routes[i].active) continue;
        if (g_routes[i].src_peer != src) continue;
        const score = @as(u32, g_routes[i].weight) * 2 + @as(u32, g_routes[i].urgency);
        if (urgency >= 200) {
            if (g_routes[i].urgency >= 200 and score > best_score) {
                best_score = score;
                best_idx = @intCast(i);
            }
        } else if (score > best_score) {
            best_score = score;
            best_idx = @intCast(i);
        }
    }
    return best_idx;
}
