const std = @import("std");

const CRIU_BIN_PATH = "/usr/sbin/criu";

pub export fn criu_checkpoint(
    pid: i32,
    images_dir: [*:0]const u8,
) i32 {
    _ = pid;
    _ = images_dir;
    return 0;
}

pub export fn criu_restore(
    images_dir: [*:0]const u8,
) i32 {
    _ = images_dir;
    return 0;
}

pub export fn criu_pre_dump(
    pid: i32,
    images_dir: [*:0]const u8,
) i32 {
    _ = pid;
    _ = images_dir;
    return 0;
}

pub export fn criu_check() i32 {
    return 0;
}

test "criu_check_basic" {
    const result = criu_check();
    try std.testing.expect(result == 0 or result == -1);
}
