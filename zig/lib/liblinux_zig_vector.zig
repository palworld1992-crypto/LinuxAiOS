const std = @import("std");

const SIMD_WIDTH_F32: usize = 4;
const SIMD_WIDTH_F64: usize = 2;
const SIMD_WIDTH_I32: usize = 4;

pub export fn liblinux_zig_f32_dot_product_simd(a: [*]f32, b: [*]f32, len: usize) callconv(.c) f32 {
    if (len == 0) return 0.0;
    var sum: f32 = 0.0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        sum += @reduce(.Add, va * vb);
    }
    while (i < len) : (i += 1) {
        sum += a[i] * b[i];
    }
    return sum;
}

pub export fn liblinux_zig_f32_normalize_simd(a: [*]f32, len: usize) callconv(.c) f32 {
    if (len == 0) return 0.0;
    var ss: f32 = 0.0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        ss += @reduce(.Add, va * va);
    }
    while (i < len) : (i += 1) {
        ss += a[i] * a[i];
    }
    return @sqrt(ss);
}

pub export fn liblinux_zig_f32_scale_simd(a: [*]f32, scale: f32, len: usize) callconv(.c) void {
    var i: usize = 0;
    const vs: @Vector(SIMD_WIDTH_F32, f32) = .{ scale, scale, scale, scale };
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const r = va * vs;
        a[i] = r[0];
        a[i + 1] = r[1];
        a[i + 2] = r[2];
        a[i + 3] = r[3];
    }
    while (i < len) : (i += 1) {
        a[i] *= scale;
    }
}

pub export fn liblinux_zig_f32_add_simd(a: [*]f32, b: [*]f32, len: usize) callconv(.c) void {
    var i: usize = 0;
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const r = va + vb;
        a[i] = r[0];
        a[i + 1] = r[1];
        a[i + 2] = r[2];
        a[i + 3] = r[3];
    }
    while (i < len) : (i += 1) {
        a[i] += b[i];
    }
}

pub export fn liblinux_zig_f32_sub_simd(a: [*]f32, b: [*]f32, len: usize) callconv(.c) void {
    var i: usize = 0;
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const r = va - vb;
        a[i] = r[0];
        a[i + 1] = r[1];
        a[i + 2] = r[2];
        a[i + 3] = r[3];
    }
    while (i < len) : (i += 1) {
        a[i] -= b[i];
    }
}

pub export fn liblinux_zig_f32_l2_distance_sq_simd(a: [*]f32, b: [*]f32, len: usize) callconv(.c) f32 {
    if (len == 0) return 0.0;
    var sum: f32 = 0.0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const d = va - vb;
        sum += @reduce(.Add, d * d);
    }
    while (i < len) : (i += 1) {
        const d = a[i] - b[i];
        sum += d * d;
    }
    return sum;
}

pub export fn liblinux_zig_f64_dot_product_simd(a: [*]f64, b: [*]f64, len: usize) callconv(.c) f64 {
    if (len == 0) return 0.0;
    var sum: f64 = 0.0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        const vb: @Vector(SIMD_WIDTH_F64, f64) = .{ b[i], b[i + 1] };
        sum += @reduce(.Add, va * vb);
    }
    while (i < len) : (i += 1) {
        sum += a[i] * b[i];
    }
    return sum;
}

pub export fn liblinux_zig_f64_normalize_simd(a: [*]f64, len: usize) callconv(.c) f64 {
    if (len == 0) return 0.0;
    var ss: f64 = 0.0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        ss += @reduce(.Add, va * va);
    }
    while (i < len) : (i += 1) {
        ss += a[i] * a[i];
    }
    return @sqrt(ss);
}

pub export fn liblinux_zig_f64_scale_simd(a: [*]f64, scale: f64, len: usize) callconv(.c) void {
    var i: usize = 0;
    const vs: @Vector(SIMD_WIDTH_F64, f64) = .{ scale, scale };
    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        const r = va * vs;
        a[i] = r[0];
        a[i + 1] = r[1];
    }
    while (i < len) : (i += 1) {
        a[i] *= scale;
    }
}

pub export fn liblinux_zig_i32_sum_simd(a: [*]i32, len: usize) callconv(.c) i32 {
    var sum: i32 = 0;
    var i: usize = 0;
    while (i + SIMD_WIDTH_I32 <= len) : (i += SIMD_WIDTH_I32) {
        const va: @Vector(SIMD_WIDTH_I32, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        sum += @reduce(.Add, va);
    }
    while (i < len) : (i += 1) {
        sum += a[i];
    }
    return sum;
}

pub export fn liblinux_zig_i32_max_simd(a: [*]i32, len: usize) callconv(.c) i32 {
    if (len == 0) return 0;
    var max_val = a[0];
    var i: usize = 0;
    while (i + SIMD_WIDTH_I32 <= len) : (i += SIMD_WIDTH_I32) {
        const va: @Vector(SIMD_WIDTH_I32, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vm = @reduce(.Max, va);
        if (vm > max_val) max_val = vm;
    }
    while (i < len) : (i += 1) {
        if (a[i] > max_val) max_val = a[i];
    }
    return max_val;
}

pub export fn liblinux_zig_i32_min_simd(a: [*]i32, len: usize) callconv(.c) i32 {
    if (len == 0) return 0;
    var min_val = a[0];
    var i: usize = 0;
    while (i + SIMD_WIDTH_I32 <= len) : (i += SIMD_WIDTH_I32) {
        const va: @Vector(SIMD_WIDTH_I32, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vm = @reduce(.Min, va);
        if (vm < min_val) min_val = vm;
    }
    while (i < len) : (i += 1) {
        if (a[i] < min_val) min_val = a[i];
    }
    return min_val;
}
