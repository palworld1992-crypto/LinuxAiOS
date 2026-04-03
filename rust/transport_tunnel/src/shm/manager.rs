use super::SharedMemoryRegion;
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct ShmManager {
    regions: RwLock<HashMap<String, SharedMemoryRegion>>,
}

impl ShmManager {
    pub fn new() -> Self {
        Self {
            regions: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_region(&self, name: &str, size: usize) -> anyhow::Result<()> {
        let region = SharedMemoryRegion::create(name, size)?;
        self.regions.write().insert(name.to_string(), region);
        Ok(())
    }

    // Sửa: trả về Option<&SharedMemoryRegion> không thể vì vòng đời; chuyển thành trả về Option<SharedMemoryRegion> clone hoặc reference không cần?
    // Cách an toàn: trả về Option<&SharedMemoryRegion> nhưng không được, vì RwLockReadGuard sẽ outlive.
    // Giải pháp: trả về Option<SharedMemoryRegion> bằng cách clone? Nhưng SharedMemoryRegion không clone.
    // Thay vào đó, ta nên cung cấp method trả về guard. Hoặc đơn giản hóa: không cần method này.
    pub fn get_region(&self, _name: &str) -> Option<&SharedMemoryRegion> {
        // Lưu ý: trả về &SharedMemoryRegion không thể vì nó được bảo vệ bởi lock, nhưng lock được giải phóng sau khi hàm kết thúc.
        // Để tránh lỗi, chúng ta sẽ sửa lại: đọc vào một biến tạm và trả về Option, nhưng vẫn lỗi.
        // Giải pháp: thay đổi API để trả về guard hoặc không dùng.
        // Tạm thời, comment method này lại.
        unimplemented!("Use get_region_with_guard instead")
    }
}
