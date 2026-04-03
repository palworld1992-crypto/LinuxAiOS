#include <iostream>
#include "ffi/browser_ffi.h"

int main() {
    std::cout << "Browser Module starting" << std::endl;
    // Khởi tạo Chromium (placeholder)
    browser_send_heartbeat();
    // Giả lập memory alert
    browser_memory_alert(800);
    // Nhận lệnh từ Rust
    browser_receive_command("discard_tab", 123);
    return 0;
}