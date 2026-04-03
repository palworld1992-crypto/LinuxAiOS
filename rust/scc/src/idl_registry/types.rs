use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, PartialEq)] // thêm PartialEq để so sánh
pub enum IDLKind {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    String,
    Array,
    Struct,
}

#[repr(C)]
pub struct IDLType {
    pub kind: IDLKind,
    pub element_type: *mut IDLType,
    pub length: usize,
    pub field_count: usize,
    pub fields: *mut *mut IDLField,
}

#[repr(C)]
pub struct IDLField {
    pub name: [c_char; 64],
    pub type_info: *mut IDLType,
    pub offset: usize,
}
