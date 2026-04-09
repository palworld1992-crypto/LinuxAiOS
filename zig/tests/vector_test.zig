const std = @import("std");
const vector = @import("zig_vector");

test "dot product identical vectors" {
    var a = [_]f32{ 1.0, 2.0, 3.0, 4.0 };
    var b = [_]f32{ 1.0, 2.0, 3.0, 4.0 };
    const result = vector.f32_dot_product_simd(&a, &b, 4);
    try std.testing.expectApproxEqRel(30.0, result, 0.001);
}

test "dot product orthogonal vectors" {
    var a = [_]f32{ 1.0, 0.0, 0.0, 0.0 };
    var b = [_]f32{ 0.0, 1.0, 0.0, 0.0 };
    const result = vector.f32_dot_product_simd(&a, &b, 4);
    try std.testing.expectApproxEqRel(0.0, result, 0.001);
}

test "dot product empty" {
    var a = [_]f32{0.0};
    var b = [_]f32{0.0};
    const result = vector.f32_dot_product_simd(&a, &b, 0);
    try std.testing.expectApproxEqRel(0.0, result, 0.001);
}

test "normalize vector" {
    var a = [_]f32{ 3.0, 4.0, 0.0, 0.0 };
    const norm = vector.f32_normalize_simd(&a, 4);
    try std.testing.expectApproxEqRel(5.0, norm, 0.001);
}

test "normalize empty" {
    var a = [_]f32{0.0};
    const norm = vector.f32_normalize_simd(&a, 0);
    try std.testing.expectApproxEqRel(0.0, norm, 0.001);
}

test "scale vector" {
    var a = [_]f32{ 1.0, 2.0, 3.0, 4.0 };
    vector.f32_scale_simd(&a, 2.0, 4);
    try std.testing.expectApproxEqRel(2.0, a[0], 0.001);
    try std.testing.expectApproxEqRel(4.0, a[1], 0.001);
    try std.testing.expectApproxEqRel(6.0, a[2], 0.001);
    try std.testing.expectApproxEqRel(8.0, a[3], 0.001);
}
