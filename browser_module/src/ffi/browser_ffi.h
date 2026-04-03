#ifndef BROWSER_FFI_H
#define BROWSER_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Gọi từ Rust: gửi lệnh discard tab
void browser_receive_command(const char* command, uint64_t param);

// Gọi từ C++: gửi heartbeat
void browser_send_heartbeat();

// Gọi từ C++: cảnh báo memory
void browser_memory_alert(uint64_t memory_used_mb);

#ifdef __cplusplus
}
#endif

#endif