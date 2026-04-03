pub mod encapsulation;
pub mod io_uring;
pub mod shm;

pub use encapsulation::TransportTunnel;
pub use encapsulation::{client_handshake, server_handshake, SessionKey, SessionManager};
pub use shm::{SharedMemoryRegion, ShmManager};
