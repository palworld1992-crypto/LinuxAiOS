use bitflags::bitflags;
use tracing::warn;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct SupportFlags: u32 {
        const ContainerMonitoring = 0b00000001;
        const HybridLibrarySupervision = 0b00000010;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AndroidSupportContext {
    pub flags: SupportFlags,
    pub started_at: u64,
    pub supervisor_id: String,
}

impl AndroidSupportContext {
    fn get_current_timestamp() -> u64 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(e) => {
                warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
                0
            }
        }
    }

    pub fn new(supervisor_id: &str) -> Self {
        Self {
            flags: SupportFlags::empty(),
            started_at: Self::get_current_timestamp(),
            supervisor_id: supervisor_id.to_string(),
        }
    }

    pub fn add_flag(&mut self, flag: SupportFlags) {
        self.flags |= flag;
    }

    pub fn remove_flag(&mut self, flag: SupportFlags) {
        self.flags &= !flag;
    }

    pub fn has_flag(&self, flag: SupportFlags) -> bool {
        self.flags.contains(flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_context_creation() -> anyhow::Result<()> {
        let ctx = AndroidSupportContext::new("sup-1");
        assert_eq!(ctx.supervisor_id, "sup-1");
        assert!(ctx.flags.is_empty());
        Ok(())
    }

    #[test]
    fn test_add_remove_flags() -> anyhow::Result<()> {
        let mut ctx = AndroidSupportContext::new("sup-1");
        ctx.add_flag(SupportFlags::ContainerMonitoring);
        assert!(ctx.has_flag(SupportFlags::ContainerMonitoring));
        ctx.remove_flag(SupportFlags::ContainerMonitoring);
        assert!(!ctx.has_flag(SupportFlags::ContainerMonitoring));
        Ok(())
    }
}
