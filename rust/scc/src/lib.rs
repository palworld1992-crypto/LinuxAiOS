pub mod blockchain;
pub mod connection;
pub mod crypto;
pub mod idl_registry;
pub mod token;
pub mod translation;
pub mod translator;
pub mod transport;
pub mod validation;

pub use blockchain::BlockchainLightClient;
pub use connection::ConnectionManager;
pub use token::CapabilityToken;
pub use token::IntentToken;
pub use token::TokenError;
pub use translation::engine::{ShmHandle, TranslationEngine, TranslationError};
pub use transport::TransportBridge;
pub use validation::{IntentValidator, Policy, ValidationError};

use std::sync::Once;

extern "C" {
    // Bọc catch_unwind để tránh panic across FFI boundary
    pub fn sccinit();
    pub fn sccfinal();
}

#[cfg(not(test))]
/// Wrapper safe để gọi sccfinal từ atexit
/// SAFETY: sccfinal() là hàm C/Ada an toàn, chỉ cleanup runtime. Được gọi qua atexit nên đảm bảo không panic.
extern "C" fn call_adafinal_wrapper() {
    // Bọc catch_unwind để đảm bảo không có panic từ Ada cross sang Rust
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: sccfinal() chỉ cleanup Ada runtime, gọi từ atexit nên process đang exit an toàn.
        unsafe {
            sccfinal();
        }
    }));
}

static ADA_INIT: Once = Once::new();

/// Khởi tạo Ada runtime. Phải được gọi trước bất kỳ lời gọi FFI nào đến Ada.
/// SAFETY: sccinit() khởi tạo Ada runtime. Phải được gọi đúng 1 lần trước mọi FFI call khác.
pub fn init_ada() {
    ADA_INIT.call_once(|| {
        // Bọc catch_unwind để tránh panic cross FFI boundary
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // SAFETY: sccinit() khởi tạo Ada runtime, gọi 1 lần duy nhất trước mọi FFI.
            unsafe {
                sccinit();
            }
        }));
        if let Err(e) = result {
            tracing::error!("FFI panic in sccinit: {:?}", e);
        }

        #[cfg(not(test))]
        {
            // Chỉ đăng ký finalizer ở runtime thực tế.
            // Trong unit test, teardown Ada có thể gây foreign exception lúc process exit.
            // SAFETY: atexit đăng ký callback an toàn khi process exit.
            unsafe {
                libc::atexit(call_adafinal_wrapper);
            }
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
