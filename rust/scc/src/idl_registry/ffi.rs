use super::types::IDLType;

#[cfg(not(any(tarpaulin, test)))]
extern "C" {
    pub fn type_mapper_map_type(t: *const IDLType) -> usize; // pub để test dùng
    pub fn layout_calculator_compute(s: *mut IDLType);
}

#[cfg(any(tarpaulin, test))]
#[inline]
pub unsafe fn type_mapper_map_type(t: *const IDLType) -> usize {
    if t.is_null() {
        return 0;
    }
    // Fallback khi không link được Ada symbols (test/coverage).
    match (*t).kind {
        super::types::IDLKind::U8 => 1,
        super::types::IDLKind::U16 => 2,
        super::types::IDLKind::U32 => 4,
        super::types::IDLKind::U64 => 8,
        super::types::IDLKind::I8 => 1,
        super::types::IDLKind::I16 => 2,
        super::types::IDLKind::I32 => 4,
        super::types::IDLKind::I64 => 8,
        super::types::IDLKind::F32 => 4,
        super::types::IDLKind::F64 => 8,
        super::types::IDLKind::String => 8,
        super::types::IDLKind::Array => (*t).length * type_mapper_map_type((*t).element_type),
        super::types::IDLKind::Struct => {
            let mut total = 0usize;
            for i in 0..(*t).field_count {
                let field = *(*t).fields.add(i);
                if !(*field).type_info.is_null() {
                    total += type_mapper_map_type((*field).type_info);
                }
            }
            total
        }
    }
}

#[cfg(any(tarpaulin, test))]
#[inline]
pub unsafe fn layout_calculator_compute(_s: *mut IDLType) {
    // Stub for test/coverage builds when Ada symbols are unavailable.
}

pub struct LayoutCalculator;

impl LayoutCalculator {
    pub fn compute_layout(struct_type: &mut IDLType) {
        // SAFETY: layout_calculator_compute is an FFI function that accepts a valid mutable pointer
        // to an IDLType. struct_type is a valid &mut reference, so the pointer is non-null and valid.
        // Wrapped in catch_unwind to prevent panics from propagating across FFI boundary.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            layout_calculator_compute(struct_type)
        }));
        if let Err(e) = result {
            tracing::error!("FFI panic in layout_calculator_compute: {:?}", e);
        }
    }
}

impl IDLType {
    pub fn size_of(&self) -> usize {
        // SAFETY: type_mapper_map_type is an FFI function that accepts a valid pointer to an IDLType.
        // self is a valid & reference, so the pointer is non-null and valid for the lifetime of this call.
        unsafe { type_mapper_map_type(self) }
    }
}
