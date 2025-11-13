use naia_shared::{DiffMask, MutReceiver, WorldChannelError};

#[test]
fn test_rwlock_reentrant_error() {
    let receiver = MutReceiver::new(8);

    // Hold a read lock
    let _guard = receiver.try_mask().expect("first lock should succeed");

    // Try to acquire write lock while read lock is held (on same thread)
    // This should fail with RwLockReentrant error
    let result = receiver.try_mask_mut();

    assert!(result.is_err());
    match result {
        Err(WorldChannelError::RwLockReentrant) => {
            // Expected error
        }
        _ => panic!("Expected RwLockReentrant error"),
    }
}

#[test]
fn test_try_mask_success() {
    let receiver = MutReceiver::new(8);

    let result = receiver.try_mask();
    assert!(result.is_ok());

    let mask = result.unwrap();
    assert!(mask.is_clear());
}

#[test]
fn test_try_diff_mask_is_clear() {
    let receiver = MutReceiver::new(8);

    // Initially should be clear
    let result = receiver.try_diff_mask_is_clear();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);

    // Mutate and check again
    receiver.try_mutate(3).expect("mutate should succeed");
    let result = receiver.try_diff_mask_is_clear();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[test]
fn test_try_mutate_success() {
    let receiver = MutReceiver::new(8);

    let result = receiver.try_mutate(5);
    assert!(result.is_ok());

    // Verify the bit was set
    let mask = receiver.try_mask().unwrap();
    assert!(!mask.is_clear());
}

#[test]
fn test_try_or_mask_success() {
    let receiver = MutReceiver::new(8);
    let mut other_mask = DiffMask::new(8);
    other_mask.set_bit(2, true);
    other_mask.set_bit(4, true);

    let result = receiver.try_or_mask(&other_mask);
    assert!(result.is_ok());

    // Verify the bits were set
    let mask = receiver.try_mask().unwrap();
    assert!(!mask.is_clear());
}

#[test]
fn test_try_clear_mask_success() {
    let receiver = MutReceiver::new(8);

    // Set some bits
    receiver.try_mutate(1).unwrap();
    receiver.try_mutate(3).unwrap();

    // Clear the mask
    let result = receiver.try_clear_mask();
    assert!(result.is_ok());

    // Verify mask is clear
    let mask = receiver.try_mask().unwrap();
    assert!(mask.is_clear());
}

#[test]
fn test_rwlock_reentrant_error_messages() {
    let error = WorldChannelError::RwLockReentrant;
    assert_eq!(
        error.to_string(),
        "RwLock is already held on current thread"
    );
}
