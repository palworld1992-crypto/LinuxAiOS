const std = @import("std");

pub export fn f32_dot_product_simd(
    a: [*]f32,
    b: [*]f32,
    len: usize,
) f32 {
    if (len == 0) return 0.0;

    var sum: f32 = 0.0;
    var i: usize = 0;

    const simd_width: usize = 4;

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(simd_width, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
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
) f32 {
    if (len == 0) return 0.0;

    var sum_squared: f32 = 0.0;
    var i: usize = 0;

    const simd_width: usize = 4;

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
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
) void {
    var i: usize = 0;
    const simd_width: usize = 4;
    const vscale: @Vector(simd_width, f32) = .{ scale, scale, scale, scale };

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
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
) void {
    var i: usize = 0;
    const simd_width: usize = 4;

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(simd_width, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
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
) void {
    var i: usize = 0;
    const simd_width: usize = 4;

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, f32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        const vb: @Vector(simd_width, f32) = .{ b[i], b[i + 1], b[i + 2], b[i + 3] };
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

pub export fn i32_sum_simd(
    a: [*]i32,
    len: usize,
) i32 {
    var sum: i32 = 0;
    var i: usize = 0;
    const simd_width: usize = 4;

    while (i + simd_width <= len) : (i += simd_width) {
        const va: @Vector(simd_width, i32) = .{ a[i], a[i + 1], a[i + 2], a[i + 3] };
        sum += @reduce(.Add, va);
    }

    while (i < len) : (i += 1) {
        sum += a[i];
    }

    return sum;
}

test "vector_simd_basic" {
    var a = [_]f32{ 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0 };
    var b = [_]f32{ 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0 };

    const dot = f32_dot_product_simd(&a, &b, 8);
    try std.testing.expect(dot > 0.0);
}
