use scc::idl_registry::{IDLField, IDLKind, IDLType, LayoutCalculator};
use std::ptr;

#[test]
fn test_idl_kind_equality() {
    assert_eq!(IDLKind::U8, IDLKind::U8);
    assert_eq!(IDLKind::U32, IDLKind::U32);
    assert_eq!(IDLKind::F64, IDLKind::F64);
    assert_eq!(IDLKind::String, IDLKind::String);
    assert_eq!(IDLKind::Array, IDLKind::Array);
    assert_eq!(IDLKind::Struct, IDLKind::Struct);
}

#[test]
fn test_idl_kind_inequality() {
    assert_ne!(IDLKind::U8, IDLKind::U16);
    assert_ne!(IDLKind::I32, IDLKind::F32);
    assert_ne!(IDLKind::String, IDLKind::Array);
}

#[test]
fn test_idl_kind_debug() {
    let kind = IDLKind::U64;
    let debug = format!("{:?}", kind);
    assert!(debug.contains("U64"));
}

#[test]
fn test_idl_kind_all_variants() {
    let kinds = [
        IDLKind::U8,
        IDLKind::U16,
        IDLKind::U32,
        IDLKind::U64,
        IDLKind::I8,
        IDLKind::I16,
        IDLKind::I32,
        IDLKind::I64,
        IDLKind::F32,
        IDLKind::F64,
        IDLKind::String,
        IDLKind::Array,
        IDLKind::Struct,
    ];

    for kind in &kinds {
        let debug = format!("{:?}", kind);
        assert!(!debug.is_empty());
    }
}

#[test]
fn test_idl_type_creation() {
    let idl_type = IDLType {
        kind: IDLKind::U32,
        element_type: ptr::null_mut(),
        length: 0,
        field_count: 0,
        fields: ptr::null_mut(),
    };

    assert_eq!(idl_type.kind, IDLKind::U32);
    assert!(idl_type.element_type.is_null());
    assert_eq!(idl_type.length, 0);
    assert_eq!(idl_type.field_count, 0);
    assert!(idl_type.fields.is_null());
}

#[test]
fn test_idl_type_array() {
    let idl_type = IDLType {
        kind: IDLKind::Array,
        element_type: ptr::null_mut(),
        length: 10,
        field_count: 0,
        fields: ptr::null_mut(),
    };

    assert_eq!(idl_type.kind, IDLKind::Array);
    assert_eq!(idl_type.length, 10);
}

#[test]
fn test_idl_field_creation() {
    use std::os::raw::c_char;

    let mut name = [0i8; 64];
    let field_name = b"test_field";
    for (i, &b) in field_name.iter().enumerate() {
        name[i] = b as c_char;
    }

    let field = IDLField {
        name,
        type_info: ptr::null_mut(),
        offset: 0,
    };

    assert_eq!(field.offset, 0);
    assert!(field.type_info.is_null());
}

#[test]
fn test_layout_calculator_exists() {
    let _calc = LayoutCalculator;
}
