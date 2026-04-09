const std = @import("std");
const linux = @import("std").os.linux;
const loader = @import("loader.zig");

/// IPC Router module - eBPF-based message routing with Hebbian learning
/// Implements route management, priority calculation, and broadcast logic
pub const RouteEntry = struct {
    src_peer: u32,
    dst_sock: u32,
    weight: u8,
    urgency: u8,
    ring_buffer_fd: i32,
    active: bool,
};

pub const RouteKey = struct {
    src: u32,
    dst: u32,
};

pub const RouterInfo = struct {
    prog_fd: i32,
    map_fd: i32,
    route_count: u32,
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

var g_route_stats: RouteStats = RouteStats{
    .total_packets = 0,
    .high_urgency = 0,
    .medium_urgency = 0,
    .low_urgency = 0,
    .dropped = 0,
    .forwarded = 0,
    .hebbian_updates = 0,
};

var g_routes: [256]RouteEntry = undefined;
var g_route_count: u32 = 0;
var g_router_info: RouterInfo = RouterInfo{ .prog_fd = -1, .map_fd = -1, .route_count = 0 };

pub export fn init_router(prog_path: [*:0]const u8) callconv(.c) i32 {
    const prog_type: u32 = 21;
    const prog_fd = loader.ebpf_load_program(prog_path, prog_type);
    if (prog_fd < 0) return -1;

    const map_fd = create_sockmap(4, 4, 256);
    if (map_fd < 0) {
        _ = loader.ebpf_close(prog_fd);
        return -1;
    }

    g_router_info.prog_fd = prog_fd;
    g_router_info.map_fd = map_fd;
    g_router_info.route_count = 0;

    return 0;
}

pub export fn create_sockmap(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    return loader.ebpf_create_sockmap(key_size, value_size, max_entries);
}

pub export fn create_route_map(key_size: u32, value_size: u32, max_entries: u32) callconv(.c) i32 {
    return loader.ebpf_create_hash_map(key_size, value_size, max_entries);
}

pub export fn update_sockmap_route(map_fd: i32, src_peer: u32, dst_sock: u32) callconv(.c) i32 {
    var key: u32 = src_peer;
    var value: u32 = dst_sock;
    return loader.ebpf_update_map_elem(map_fd, &key, &value, 0);
}

pub export fn remove_sockmap_route(map_fd: i32, src_peer: u32) callconv(.c) i32 {
    var key: u32 = src_peer;
    return loader.ebpf_delete_map_elem(map_fd, &key);
}

pub export fn attach_sockmap_prog(map_fd: i32, prog_fd: i32) callconv(.c) i32 {
    return loader.ebpf_attach_sockmap(map_fd, prog_fd);
}

pub export fn add_route(map_fd: i32, src: u32, dst: u32, weight: u8, urgency: u8, ring_fd: i32) callconv(.c) i32 {
    _ = map_fd;
    if (g_route_count >= 256) return -1;

    g_routes[g_route_count] = RouteEntry{
        .src_peer = src,
        .dst_sock = dst,
        .weight = weight,
        .urgency = urgency,
        .ring_buffer_fd = ring_fd,
        .active = true,
    };
    g_route_count += 1;
    return 0;
}

pub export fn remove_route(map_fd: i32, src: u32) callconv(.c) i32 {
    _ = map_fd;
    var i: u32 = 0;
    while (i < g_route_count) : (i += 1) {
        if (g_routes[i].src_peer == src) {
            g_routes[i].active = false;
            return 0;
        }
    }
    return -1;
}

pub export fn get_route_priority(src_peer: u32, signal_type: u8, urgency: u8) callconv(.c) u8 {
    _ = src_peer;
    var priority = urgency;
    if (signal_type == 1) {
        priority = @min(@as(u16, priority) + 50, 255);
    }
    return @intCast(priority);
}

pub export fn should_forward(token_urgency: u8, route_weight: u8, signal_type: u8) callconv(.c) bool {
    if (token_urgency >= 200) return true;
    if (signal_type == 1 and route_weight > 128) return true;
    if (route_weight > 64) return true;
    return false;
}

pub export fn hebbian_update_on_success(
    route_weight: *u8,
    urgency: u8,
    signal_type: u8,
) callconv(.c) void {
    var increment: u8 = 1;
    if (urgency >= 200) {
        increment = 2;
    }
    if (signal_type == 1) {
        increment += 1;
    }

    const new_weight = route_weight.* +% increment;
    if (new_weight > 255) {
        route_weight.* = 255;
    } else {
        route_weight.* = new_weight;
    }
    g_route_stats.hebbian_updates += 1;
}

pub export fn calculate_weight(urgency: u8, queue_depth: u32, bandwidth_available: u32) callconv(.c) u8 {
    var base_weight: u8 = 0;
    if (urgency >= 200) {
        base_weight = 200;
    } else if (urgency >= 100) {
        base_weight = 128;
    } else {
        base_weight = 64;
    }

    const queue_factor = @min(queue_depth / 1000, 50);
    const bandwidth_factor = @min(bandwidth_available / 1000000, 50);

    const total = base_weight +% queue_factor +% bandwidth_factor;
    return @min(total, 255);
}

pub export fn get_route_stats() callconv(.c) ?*RouteStats {
    return &g_route_stats;
}

pub export fn reset_stats() callconv(.c) void {
    g_route_stats.total_packets = 0;
    g_route_stats.high_urgency = 0;
    g_route_stats.medium_urgency = 0;
    g_route_stats.low_urgency = 0;
    g_route_stats.dropped = 0;
    g_route_stats.forwarded = 0;
    g_route_stats.hebbian_updates = 0;
}

pub export fn find_best_route(map_fd: i32, src: u32, urgency: u8) callconv(.c) i32 {
    if (map_fd >= 0) {
        var key: u32 = src;
        var value: [256]RouteEntry = undefined;
        @memset(@as([*]u8, @ptrCast(&value))[0..@sizeOf(@TypeOf(value))], 0);

        const result = loader.ebpf_lookup_map_elem(map_fd, &key, &value);
        if (result >= 0) {
            var best_score: u32 = 0;
            var best_idx: i32 = -1;
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

pub export fn should_broadcast(signal_type: u8, route_weight: u8) callconv(.c) bool {
    return signal_type == 1 and route_weight > 128;
}

pub export fn get_max_broadcast_modules() callconv(.c) u8 {
    return 5;
}

pub export fn get_urgency_threshold_high() callconv(.c) u8 {
    return 200;
}

pub export fn get_urgency_threshold_medium() callconv(.c) u8 {
    return 100;
}

pub export fn get_broadcast_weight_threshold() callconv(.c) u8 {
    return 128;
}

pub export fn decay_route_weight(route_weight: *u8, decay_factor: u8) callconv(.c) void {
    if (decay_factor > 0 and decay_factor < 100) {
        const decay = @as(u16, route_weight.*) * (100 - decay_factor) / 100;
        route_weight.* = @truncate(decay);
    }
}

pub export fn is_route_active(route_weight: u8, last_used_timestamp: u64) callconv(.c) bool {
    _ = last_used_timestamp;
    return route_weight > 0;
}

pub export fn router_is_supported() callconv(.c) bool {
    return loader.ebpf_is_supported();
}

pub export fn get_route_count() callconv(.c) u32 {
    return g_route_count;
}

pub export fn clear_all_routes() callconv(.c) void {
    g_route_count = 0;
}

pub export fn get_hebbian_learning_rate() callconv(.c) u8 {
    return 1;
}

pub export fn set_hebbian_learning_rate(rate: u8) callconv(.c) void {
    _ = rate;
}

test "ipc_router_basic" {
    try std.testing.expectEqual(@as(u8, 200), get_urgency_threshold_high());
    try std.testing.expectEqual(@as(u8, 100), get_urgency_threshold_medium());
    try std.testing.expectEqual(@as(u8, 128), get_broadcast_weight_threshold());
    try std.testing.expectEqual(@as(u8, 5), get_max_broadcast_modules());

    try std.testing.expect(should_broadcast(1, 200));
    try std.testing.expect(!should_broadcast(0, 200));
    try std.testing.expect(!should_broadcast(1, 100));

    try std.testing.expectEqual(@as(u8, 200), calculate_weight(200, 0, 0));
    try std.testing.expectEqual(@as(u8, 128), calculate_weight(100, 0, 0));
    try std.testing.expectEqual(@as(u8, 64), calculate_weight(50, 0, 0));
}
