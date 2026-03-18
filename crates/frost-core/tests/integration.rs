use frost_core::{sha256}; // Treat the crate as an external dependency

#[test]
fn test_public_interface() {
    let result = sha256(b"integration_test");
    assert!(result.len() == 32);
}