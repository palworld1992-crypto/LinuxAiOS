#include "browser_ffi.h"
#include <iostream>

void browser_receive_command(const char* command, uint64_t param) {
    std::cout << "Browser received command: " << command << " param=" << param << std::endl;
    // Thực tế sẽ gọi Chromium API
}

void browser_send_heartbeat() {
    // Gửi tín hiệu đến Rust supervisor (có thể qua socket hoặc shared memory)
    std::cout << "Browser heartbeat" << std::endl;
}

void browser_memory_alert(uint64_t memory_used_mb) {
    std::cout << "Browser memory alert: " << memory_used_mb << " MB" << std::endl;
}