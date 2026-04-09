use anyhow::bail;
use anyhow::Result;
use common::CommonError;
use std::io;

#[test]
fn test_common_error_io() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err: CommonError = CommonError::from(io_err);
    match err {
        CommonError::Io(e) => assert!(e.to_string().contains("file not found")),
        _ => panic!("Expected Io variant"),
    }
}

#[test]
fn test_common_error_serialization() -> Result<()> {
    let result: Result<serde_json::Value, _> = serde_json::from_str("invalid json");
    let json_err = match result {
        Err(e) => e,
        Ok(_) => bail!("Expected parsing error"),
    };
    let err: CommonError = CommonError::from(json_err);
    match err {
        CommonError::Serialization(e) => assert!(!e.to_string().is_empty()),
        _ => panic!("Expected Serialization variant"),
    }
    Ok(())
}

#[test]
fn test_common_error_invalid_argument() {
    let err = CommonError::InvalidArgument("must be positive".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("must be positive"));
}

#[test]
fn test_common_error_display_io() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let err: CommonError = CommonError::from(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("access denied"));
}

#[test]
fn test_common_error_display_serialization() -> Result<()> {
    let bad_json = "not json at all";
    let result: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
    let json_err = match result {
        Err(e) => e,
        Ok(_) => bail!("Expected parsing error"),
    };
    let err: CommonError = CommonError::from(json_err);
    let msg = format!("{}", err);
    assert!(!msg.is_empty());
    Ok(())
}

#[test]
fn test_common_error_debug() {
    let err = CommonError::InvalidArgument("test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("InvalidArgument"));
}

#[test]
fn test_common_error_from_io_result() -> Result<()> {
    let result: Result<(), io::Error> = Err(io::Error::other("other error"));
    let err: CommonError = match result {
        Err(e) => e.into(),
        Ok(_) => bail!("Expected Err"),
    };
    match err {
        CommonError::Io(e) => assert!(e.to_string().contains("other error")),
        _ => panic!("Expected Io variant"),
    }
    Ok(())
}

#[test]
fn test_common_error_from_json_result() -> Result<()> {
    let result: Result<serde_json::Value, _> = serde_json::from_str("{broken");
    let err: CommonError = match result {
        Err(e) => e.into(),
        Ok(_) => bail!("Expected Err"),
    };
    match err {
        CommonError::Serialization(e) => assert!(e.to_string().contains("expected value")),
        _ => panic!("Expected Serialization variant"),
    }
    Ok(())
}
