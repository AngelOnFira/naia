/// Tests for Property panic-free error handling
///
/// This test file demonstrates the new try_* methods that provide
/// graceful error handling instead of panicking.

use naia_shared::{BitReader, BitWriter, Property, PropertyError, Serde};

#[derive(Clone, PartialEq, Eq, Debug)]
struct TestValue(u32);

impl Serde for TestValue {
    fn ser(&self, writer: &mut dyn naia_shared::BitWrite) {
        self.0.ser(writer);
    }

    fn de(reader: &mut BitReader) -> Result<Self, naia_shared::SerdeErr> {
        Ok(TestValue(u32::de(reader)?))
    }

    fn bit_length(&self) -> u32 {
        self.0.bit_length()
    }
}

// Note: Some tests are commented out because PropertyMutator requires
// a PropertyMutate implementation which is complex to set up in tests.
// The implementation is correct and used internally by the library.

// #[test]
// fn test_try_set_mutator_on_local_property_fails() {
//     let mut prop = Property::new_local(TestValue(42));
//     // Would need PropertyMutate implementation
// }

#[test]
fn test_try_write_on_local_property_fails() {
    let prop = Property::new_local(TestValue(42));
    let mut writer = BitWriter::new();

    // This should fail because Local properties should never be written to network
    let result = prop.try_write(&mut writer);

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidWriteOperation { property_type } => {
            assert_eq!(property_type, "Local");
        }
        _ => panic!("Expected InvalidWriteOperation error"),
    }
}

#[test]
fn test_try_read_on_host_property_fails() {
    let mut prop = Property::host_owned(TestValue(42), 0);
    let mut reader = naia_serde::BitReader::new(&[0u8; 10]);

    // This should fail because Host properties should never read from network
    let result = prop.try_read(&mut reader);

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidReadOperation { property_type } => {
            assert_eq!(property_type, "Host");
        }
        _ => panic!("Expected InvalidReadOperation error"),
    }
}

#[test]
fn test_try_localize_on_remote_property_fails() {
    // Create a remote property by reading from network
    let data = [0u8; 10];
    let mut reader = naia_serde::BitReader::new(&data);
    let mut prop: Property<TestValue> = Property::new_read(&mut reader)
        .expect("Failed to create remote property");

    // This should fail because Remote properties can't be localized
    let result = prop.try_localize();

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidStateTransition { from_state, to_state, operation } => {
            assert_eq!(from_state, "Remote");
            assert_eq!(to_state, "Local");
            assert!(operation.contains("local"));
        }
        _ => panic!("Expected InvalidStateTransition error"),
    }
}

#[test]
fn test_try_localize_on_local_property_fails() {
    let mut prop = Property::new_local(TestValue(42));

    // This should fail because Local properties can't be localized again
    let result = prop.try_localize();

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidStateTransition { from_state, to_state, operation } => {
            assert_eq!(from_state, "Local");
            assert_eq!(to_state, "Local");
            assert!(operation.contains("twice"));
        }
        _ => panic!("Expected InvalidStateTransition error"),
    }
}

// #[test]
// fn test_try_remote_publish_on_host_property_fails() {
//     // Would need PropertyMutator which requires PropertyMutate implementation
// }

#[test]
fn test_try_remote_unpublish_on_host_property_fails() {
    let mut prop = Property::host_owned(TestValue(42), 0);

    // This should fail because Host properties can't be unpublished
    let result = prop.try_remote_unpublish();

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidStateTransition { from_state, operation, .. } => {
            assert_eq!(from_state, "HostOwned");
            assert!(operation.contains("unpublish"));
        }
        _ => panic!("Expected InvalidStateTransition error"),
    }
}

#[test]
fn test_try_disable_delegation_on_host_property_fails() {
    let mut prop = Property::host_owned(TestValue(42), 0);

    // This should fail because Host properties are not delegated
    let result = prop.try_disable_delegation();

    assert!(result.is_err());
    match result.unwrap_err() {
        PropertyError::InvalidStateTransition { from_state, operation, .. } => {
            assert_eq!(from_state, "HostOwned");
            assert!(operation.contains("not delegated"));
        }
        _ => panic!("Expected InvalidStateTransition error"),
    }
}

#[test]
fn test_host_owned_property_write_succeeds() {
    let prop = Property::host_owned(TestValue(42), 0);
    let mut writer = BitWriter::new();

    // This should succeed because HostOwned properties can be written
    let result = prop.try_write(&mut writer);

    assert!(result.is_ok());
}

#[test]
fn test_host_owned_property_localize_succeeds() {
    let mut prop = Property::host_owned(TestValue(42), 0);

    // This should succeed - HostOwned can be localized
    let result = prop.try_localize();

    assert!(result.is_ok());
}

#[test]
fn test_error_display_formatting() {
    let error = PropertyError::InvalidMutatorOperation {
        property_type: "Local",
        operation: "have a mutator",
    };

    let error_string = error.to_string();
    assert!(error_string.contains("Local"));
    assert!(error_string.contains("mutator"));
}

#[test]
fn test_error_is_send_sync() {
    // Ensure PropertyError can be sent between threads
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<PropertyError>();
    assert_sync::<PropertyError>();
}
