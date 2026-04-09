mod ffi;
mod types;

pub use ffi::LayoutCalculator;
pub use types::{IDLField, IDLKind, IDLType};

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_layout_calculator() {
        let mut t = IDLType {
            kind: IDLKind::U32,
            element_type: std::ptr::null_mut(),
            length: 0,
            field_count: 0,
            fields: std::ptr::null_mut(),
        };
        LayoutCalculator::compute_layout(&mut t);
    }

    #[test]
    fn test_type_mapper() {
        let u32_type = IDLType {
            kind: IDLKind::U32,
            element_type: ptr::null_mut(),
            length: 0,
            field_count: 0,
            fields: std::ptr::null_mut(),
        };
        // SAFETY: `u32_type` is a valid IDLType with all fields properly initialized.
        // The FFI function `type_mapper_map_type` only reads these fields and does
        // not cause undefined behavior.
        let result = unsafe { ffi::type_mapper_map_type(&u32_type) };
        assert_eq!(result, 4);
    }
}
