// rust/scc/tests/common/mod.rs
//! Common test utilities for SCC tests
//!
//! NOTE: libscc.so requires liboqs_wrapper.so to be loaded at runtime.
//! Use the run_tests.sh script to run tests with LD_PRELOAD set correctly:
//!   ./rust/scc/tests/run_tests.sh --test crypto_test
//!
//! Or set LD_PRELOAD manually:
//!   LD_PRELOAD=/path/to/spark/lib/liboqs_wrapper.so cargo test

use once_cell::sync::Lazy;

static ADA_RUNTIME: Lazy<()> = Lazy::new(|| {
    scc::init_ada();
});

pub fn init() {
    Lazy::force(&ADA_RUNTIME);
}
