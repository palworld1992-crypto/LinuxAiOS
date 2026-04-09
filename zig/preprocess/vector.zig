const std = @import("std");

const SIMD_WIDTH_F32: usize = 4;
const SIMD_WIDTH_I32: usize = 4;
const SIMD_WIDTH_F64: usize = 2;

pub export fn f32_dot_product_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) callconv(.c) f32 {
    if (len == 0) return 0.0;

    var sum: f32 = 0.0;
    var i: usize = 0;

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const prod = va * vb;
        sum += @reduce(.Add, prod);
    }

    while (i < len) : (i += 1) {
        sum += a[i] * b[i];
    }

    return sum;
}

pub export fn f32_normalize_simd(
    a: [*]f32,
    len: usize,
) callconv(.c) f32 {
    if (len == 0) return 0.0;

    var sum_squared: f32 = 0.0;
    var i: usize = 0;

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const squared = va * va;
        sum_squared += @reduce(.Add, squared);
    }

    while (i < len) : (i += 1) {
        sum_squared += a[i] * a[i];
    }

    return @sqrt(sum_squared);
}

pub export fn f32_scale_simd(
    a: [*]f32,
    scale: f32,
    len: usize,
) callconv(.c) void {
    var i: usize = 0;
    const vscale: @Vector(SIMD_WIDTH_F32, f32) = .{ scale, scale, scale, scale };

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const result = va * vscale;
        a[i] = result[0];
        a[i + 1] = result[1];
        a[i + 2] = result[2];
        a[i + 3] = result[3];
    }

    while (i < len) : (i += 1) {
        a[i] *= scale;
    }
}

pub export fn f32_add_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) callconv(.c) void {
    var i: usize = 0;

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const result = va + vb;
        a[i] = result[0];
        a[i + 1] = result[1];
        a[i + 2] = result[2];
        a[i + 3] = result[3];
    }

    while (i < len) : (i += 1) {
        a[i] += b[i];
    }
}

pub export fn f32_sub_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) callconv(.c) void {
    var i: usize = 0;

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const result = va - vb;
        a[i] = result[0];
        a[i + 1] = result[1];
        a[i + 2] = result[2];
        a[i + 3] = result[3];
    }

    while (i < len) : (i += 1) {
        a[i] -= b[i];
    }
}

pub export fn f32_l2_distance_sq_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) callconv(.c) f32 {
    if (len == 0) return 0.0;

    var sum: f32 = 0.0;
    var i: usize = 0;

    while (i + SIMD_WIDTH_F32 <= len) : (i += SIMD_WIDTH_F32) {
        const va: @Vector(SIMD_WIDTH_F32, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(SIMD_WIDTH_F32, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
        const diff = va - vb;
        const squared = diff * diff;
        sum += @reduce(.Add, squared);
    }

    while (i < len) : (i += 1) {
        const d = a[i] - b[i];
        sum += d * d;
    }

    return sum;
}

pub export fn f64_dot_product_simd(
    a: [*]f64,
    b: [*]f64,
    len: usize,
) callconv(.c) f64 {
    if (len == 0) return 0.0;

    var sum: f64 = 0.0;
    var i: usize = 0;

    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        const vb: @Vector(SIMD_WIDTH_F64, f64) = .{ b[i], b[i + 1] };
        const prod = va * vb;
        sum += @reduce(.Add, prod);
    }

    while (i < len) : (i += 1) {
        sum += a[i] * b[i];
    }

    return sum;
}

pub export fn f64_normalize_simd(
    a: [*]f64,
    len: usize,
) callconv(.c) f64 {
    if (len == 0) return 0.0;

    var sum_squared: f64 = 0.0;
    var i: usize = 0;

    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        const squared = va * va;
        sum_squared += @reduce(.Add, squared);
    }

    while (i < len) : (i += 1) {
        sum_squared += a[i] * a[i];
    }

    return @sqrt(sum_squared);
}

pub export fn f64_scale_simd(
    a: [*]f64,
    scale: f64,
    len: usize,
) callconv(.c) void {
    var i: usize = 0;
    const vscale: @Vector(SIMD_WIDTH_F64, f64) = .{ scale, scale };

    while (i + SIMD_WIDTH_F64 <= len) : (i += SIMD_WIDTH_F64) {
        const va: @Vector(SIMD_WIDTH_F64, f64) = .{ a[i], a[i + 1] };
        const result = va * vscale;
        a[i] = result[0];
        a[i + 1] = result[1];
    }

    while (i < len) : (i += 1) {
        a[i] *= scale;
    }
}

pub export fn i32_sum_simd(
    a: [*]i32,
    len: usize,
) callconv(.c) i32 {
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

pub export fn i32_max_simd(
    a: [*]i32,
    len: usize,
) callconv(.c) i32 {
    if (len == 0) return 0;

    var max_val: i32 = a[0];
    var i: usize = 0;

    while (i + SIMD_WIDTH_I32 <= len) : (i += SIMD_WIDTH_I32) {
        const va: @Vector(SIMD_WIDTH_I32, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vmax = @reduce(.Max, va);
        if (vmax > max_val) max_val = vmax;
    }

    while (i < len) : (i += 1) {
        if (a[i] > max_val) max_val = a[i];
    }

    return max_val;
}

pub export fn i32_min_simd(
    a: [*]i32,
    len: usize,
) callconv(.c) i32 {
    if (len == 0) return 0;

    var min_val: i32 = a[0];
    var i: usize = 0;

    while (i + SIMD_WIDTH_I32 <= len) : (i += SIMD_WIDTH_I32) {
        const va: @Vector(SIMD_WIDTH_I32, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vmin = @reduce(.Min, va);
        if (vmin < min_val) min_val = vmin;
    }

    while (i < len) : (i += 1) {
        if (a[i] < min_val) min_val = a[i];
    }

    return min_val;
}

test "vector_simd_basic" {
    var a = [_]f32{ 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0 };
    var b = [_]f32{ 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0 };

    const dot = f32_dot_product_simd(&a, &b, 8);
    try std.testing.expect(dot > 0.0);
}
