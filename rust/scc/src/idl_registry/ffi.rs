use super::types::IDLType;

#[cfg(not(tarpaulin))]
extern "C" {
    pub fn type_mapper_map_type(t: *const IDLType) -> usize; // pub để test dùng
    pub fn layout_calculator_compute(s: *mut IDLType);
}

#[cfg(tarpaulin)]
#[inline]
unsafe fn type_mapper_map_type(t: *const IDLType) -> usize {
    if t.is_null() {
        return 0;
    }
    // Fallback đơn giản cho coverage run khi không link được Ada symbols.
    0
}

#[cfg(tarpaulin)]
#[inline]
unsafe fn layout_calculator_compute(_s: *mut IDLType) {}

pub struct LayoutCalculator;

impl LayoutCalculator {
    pub fn compute_layout(struct_type: &mut IDLType) {
        unsafe { layout_calculator_compute(struct_type) }
    }
}

impl IDLType {
    pub fn size_of(&self) -> usize {
        unsafe { type_mapper_map_type(self) }
    }
}
