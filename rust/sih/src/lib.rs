//! System Intelligence Hub – tri thức, hardware collector, recommender AI

pub mod main_component;
pub mod supervisor;

pub use main_component::SihMain;
pub use supervisor::SihSupervisor;

pub fn init() {}
