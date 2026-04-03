use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub supervisor_type: SupervisorType, // Thêm loại để định danh đúng 7 thực thể
    pub public_key_kyber: Vec<u8>,
    pub public_key_dilithium: Vec<u8>,
    pub is_standby: bool,
    pub registered_at: u64,
}

pub struct SupervisorRegistry {
    // Map theo Type để dễ dàng truy xuất đúng supervisor đích
    supervisors: RwLock<HashMap<SupervisorType, SupervisorInfo>>,
    // Standby có thể lưu riêng hoặc dùng ID để phân biệt
    standby_nodes: RwLock<HashMap<u64, SupervisorInfo>>,
    next_id: RwLock<u64>,
}

impl SupervisorRegistry {
    pub fn new() -> Self {
        Self {
            supervisors: RwLock::new(HashMap::new()),
            standby_nodes: RwLock::new(HashMap::new()),
            next_id: RwLock::new(100), // ID thấp dành cho 7 Supervisor chính
        }
    }

    // Đăng ký chính thức cho 7 Supervisor cốt lõi
    pub fn register_core(
        &self,
        s_type: SupervisorType,
        info: SupervisorInfo,
    ) -> Result<u64, &'static str> {
        let mut supervisors = self.supervisors.write();
        if supervisors.contains_key(&s_type) {
            return Err("Core Supervisor already registered");
        }

        let id = s_type as u64;
        let mut final_info = info;
        final_info.id = id;
        final_info.supervisor_type = s_type;
        final_info.is_standby = false;

        supervisors.insert(s_type, final_info);
        Ok(id)
    }

    pub fn get_by_type(&self, s_type: SupervisorType) -> Option<SupervisorInfo> {
        self.supervisors.read().get(&s_type).cloned()
    }

    pub fn update_keys(&self, s_type: SupervisorType, kyber: Vec<u8>, dilithium: Vec<u8>) -> bool {
        let mut supervisors = self.supervisors.write();
        if let Some(info) = supervisors.get_mut(&s_type) {
            info.public_key_kyber = kyber;
            info.public_key_dilithium = dilithium;
            true
        } else {
            false
        }
    }

    pub fn create_standby(&self, s_type: SupervisorType) -> Option<u64> {
        // Chỉ tạo standby nếu core đã tồn tại
        let _core = self.get_by_type(s_type)?;

        // Sinh khóa lượng tử (Sử dụng module scc của bạn)
        let (kyber_pub, _) = scc::crypto::kyber_keypair().ok()?;
        let (dilithium_pub, _) = scc::crypto::dilithium_keypair().ok()?;

        let mut next_id = self.next_id.write();
        let new_id = *next_id;
        *next_id += 1;

        let standby = SupervisorInfo {
            id: new_id,
            supervisor_type: s_type,
            public_key_kyber: kyber_pub.to_vec(),
            public_key_dilithium: dilithium_pub.to_vec(),
            is_standby: true,
            registered_at: common::utils::current_timestamp_ms(),
        };

        self.standby_nodes.write().insert(new_id, standby);
        Some(new_id)
    }

    pub fn list_all_cores(&self) -> Vec<SupervisorInfo> {
        self.supervisors.read().values().cloned().collect()
    }
}
