use super::SharedMemoryRegion;
use dashmap::DashMap;

pub struct ShmManager {
    regions: DashMap<String, SharedMemoryRegion>,
}

impl Default for ShmManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShmManager {
    pub fn new() -> Self {
        Self {
            regions: DashMap::new(),
        }
    }

    pub fn create_region(&self, name: &str, size: usize) -> anyhow::Result<()> {
        let region = SharedMemoryRegion::create(name, size)?;
        self.regions.insert(name.to_string(), region);
        Ok(())
    }

    // TODO(Phase 4): Implement get_region_with_guard that returns a guard instead of reference
    // to avoid lifetime issues with DashMap. Current signature cannot return &SharedMemoryRegion
    // because the lock guard would be dropped at function end.
    pub fn get_region(&self, _name: &str) -> Option<&SharedMemoryRegion> {
        unimplemented!("Phase 4 will implement get_region_with_guard returning proper lock guard")
    }
}
