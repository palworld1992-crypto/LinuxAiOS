//! FFI bindings to Zig functions - re-exports from modular structure.
//! Phase 3: Separated into individual modules for maintainability.
//! See zig_bindings/ directory for implementation details.

pub mod ebpf_loader;
pub mod cgroup;
pub mod iouring;
pub mod criu;
pub mod cpu_pinning;
pub mod snn_ebpf;

pub use ebpf_loader::*;
pub use cgroup::*;
pub use iouring::*;
pub use criu::*;
pub use cpu_pinning::*;
pub use snn_ebpf::*;
