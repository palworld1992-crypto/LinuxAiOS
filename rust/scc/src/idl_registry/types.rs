use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, PartialEq)]
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

impl std::fmt::Debug for IDLType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IDLType")
            .field("kind", &self.kind)
            .field("element_type", &self.element_type)
            .field("length", &self.length)
            .field("field_count", &self.field_count)
            .field("fields", &self.fields)
            .finish()
    }
}

#[repr(C)]
pub struct IDLField {
    pub name: [c_char; 64],
    pub type_info: *mut IDLType,
    pub offset: usize,
}

impl std::fmt::Debug for IDLField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IDLField")
            .field("name", &self.name)
            .field("type_info", &self.type_info)
            .field("offset", &self.offset)
            .finish()
    }
}
