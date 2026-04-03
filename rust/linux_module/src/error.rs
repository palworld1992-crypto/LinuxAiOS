use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinuxModuleError {
    #[error("Tensor pool error: {0}")]
    TensorPool(#[from] crate::tensor::TensorPoolError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("eBPF error: {0}")]
    Ebpf(String),

    #[error("Cgroup error: {0}")]
    Cgroup(String),

    #[error("Memory tiering error: {0}")]
    MemoryTiering(String),

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("FFI error: {0}")]
    Ffi(String),
}
