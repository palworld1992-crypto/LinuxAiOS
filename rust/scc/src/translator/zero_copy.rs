use crate::idl_registry::{IDLKind, IDLType, LayoutCalculator};
use std::ptr;

#[repr(C)]
pub struct ShmDescriptor {
    pub ptr: *mut u8,
    pub len: usize,
    pub fd: i32,
}

impl ShmDescriptor {
    pub fn new(ptr: *mut u8, len: usize, fd: i32) -> Self {
        Self { ptr, len, fd }
    }
}

pub struct Translator;

impl Translator {
    pub fn to_descriptor<T>(_data: &T, layout: &mut IDLType) -> Option<ShmDescriptor> {
        if layout.kind == IDLKind::Struct && layout.fields.is_null() {
            LayoutCalculator::compute_layout(layout);
        }
        let size = layout.size_of();
        if size == 0 {
            return None;
        }
        // TODO: thực tế copy dữ liệu vào shm
        Some(ShmDescriptor::new(ptr::null_mut(), size, -1))
    }

    pub fn from_descriptor<T>(_desc: &ShmDescriptor, _layout: &IDLType) -> Option<T> {
        None
    }
}
