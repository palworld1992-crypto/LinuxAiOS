pub mod blockchain;
pub mod connection;
pub mod crypto;
pub mod idl_registry;
pub mod token;
pub mod translator;
pub mod transport;
pub mod validation;

pub use blockchain::BlockchainLightClient;
pub use connection::ConnectionManager;
pub use token::CapabilityToken;
pub use transport::TransportBridge;

use std::sync::Once;

extern "C" {
    // Đổi adainit thành sccinit theo kết quả nm
    pub fn sccinit();
    // Đổi adafinal thành sccfinal (GNAT mặc định dùng [Library_Name]final)
    pub fn sccfinal();
}

#[cfg(not(test))]
/// Wrapper safe để gọi sccfinal từ atexit
extern "C" fn call_adafinal_wrapper() {
    unsafe {
        sccfinal();
    }
}

static ADA_INIT: Once = Once::new();

/// Khởi tạo Ada runtime. Phải được gọi trước bất kỳ lời gọi FFI nào đến Ada.
pub fn init_ada() {
    ADA_INIT.call_once(|| {
        unsafe {
            // Gọi hàm khởi tạo thực tế của project scc
            sccinit();
        }
        #[cfg(not(test))]
        unsafe {
            // Chỉ đăng ký finalizer ở runtime thực tế.
            // Trong unit test, teardown Ada có thể gây foreign exception lúc process exit.
            libc::atexit(call_adafinal_wrapper);
        }
    });
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use std::sync::Once;

    static TEST_INIT: Once = Once::new();

    pub fn init_test() {
        TEST_INIT.call_once(|| {
            init_ada();
        });
    }
}
