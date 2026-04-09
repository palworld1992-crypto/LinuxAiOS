use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// Định nghĩa các loại Supervisor theo thiết kế (Mục VI)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SupervisorType {
    Linux = 1,
    Windows = 2,
    Android = 3,
    Sih = 4,
    SystemHost = 5,
    Browser = 6,
    AdaptiveInterface = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorInfo {
    pub id: u64,
    pub supervisor_type: SupervisorType,
    pub public_key_kyber: Vec<u8>,
    pub public_key_dilithium: Vec<u8>,
    pub is_standby: bool,
    pub registered_at: u64,
}

pub struct SupervisorRegistry {
    supervisors: DashMap<SupervisorType, SupervisorInfo>,
    standby_nodes: DashMap<u64, SupervisorInfo>,
    next_id: AtomicU64,
}

impl Default for SupervisorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SupervisorRegistry {
    pub fn new() -> Self {
        Self {
            supervisors: DashMap::new(),
            standby_nodes: DashMap::new(),
            next_id: AtomicU64::new(100),
        }
    }

    pub fn register_core(
        &self,
        s_type: SupervisorType,
        info: SupervisorInfo,
    ) -> Result<u64, &'static str> {
        if self.supervisors.contains_key(&s_type) {
            return Err("Core Supervisor already registered");
        }

        let id = s_type as u64;
        let mut final_info = info;
        final_info.id = id;
        final_info.supervisor_type = s_type;
        final_info.is_standby = false;

        self.supervisors.insert(s_type, final_info);
        Ok(id)
    }

    pub fn get_by_type(&self, s_type: SupervisorType) -> Option<SupervisorInfo> {
        self.supervisors.get(&s_type).map(|r| r.value().clone())
    }

    pub fn update_keys(&self, s_type: SupervisorType, kyber: Vec<u8>, dilithium: Vec<u8>) -> bool {
        if let Some(mut info) = self.supervisors.get_mut(&s_type) {
            info.public_key_kyber = kyber;
            info.public_key_dilithium = dilithium;
            true
        } else {
            false
        }
    }

    pub fn create_standby(&self, s_type: SupervisorType) -> Option<u64> {
        let _core = self.get_by_type(s_type)?;

        let (kyber_pub, _) = scc::crypto::kyber_keypair().ok()?;
        let (dilithium_pub, _) = scc::crypto::dilithium_keypair().ok()?;

        let new_id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let standby = SupervisorInfo {
            id: new_id,
            supervisor_type: s_type,
            public_key_kyber: kyber_pub.to_vec(),
            public_key_dilithium: dilithium_pub.to_vec(),
            is_standby: true,
            registered_at: common::utils::current_timestamp_ms(),
        };

        self.standby_nodes.insert(new_id, standby);
        Some(new_id)
    }

    pub fn list_all_cores(&self) -> Vec<SupervisorInfo> {
        self.supervisors.iter().map(|r| r.value().clone()).collect()
    }
}
