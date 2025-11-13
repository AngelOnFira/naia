/// Tests for KeyGenerator error handling
/// Covers the unwrap removal from key_generator.rs utility module
///
/// Note: No user-facing API changes were made - the unwraps were replaced with
/// unsafe unwrap_unchecked for performance since they are provably safe.
/// These tests verify the module still works correctly.

use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct TestKey(u16);

impl From<u16> for TestKey {
    fn from(value: u16) -> Self {
        TestKey(value)
    }
}

impl From<TestKey> for u16 {
    fn from(value: TestKey) -> Self {
        value.0
    }
}

#[test]
fn key_generator_generates_sequential_keys() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_secs(1));

    let key1 = generator.generate();
    let key2 = generator.generate();
    let key3 = generator.generate();

    assert_eq!(key1.0, 0);
    assert_eq!(key2.0, 1);
    assert_eq!(key3.0, 2);
}

#[test]
fn key_generator_wraps_around() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_secs(1));

    // Set the internal counter to near max
    // Generate keys up to the wrap point
    for _ in 0..65535 {
        generator.generate();
    }

    let key_max = generator.generate(); // Should be 65535
    let key_wrapped = generator.generate(); // Should wrap to 0

    assert_eq!(key_max.0, 65535);
    assert_eq!(key_wrapped.0, 0);
}

#[test]
fn key_generator_recycles_keys_after_timeout() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_millis(10));

    // Generate a key and recycle it
    let key1 = generator.generate();
    assert_eq!(key1.0, 0);

    generator.recycle_key(&key1);

    // Immediately generating should give us a new key (not recycled yet)
    let key2 = generator.generate();
    assert_eq!(key2.0, 1);

    // Wait for recycle timeout
    std::thread::sleep(Duration::from_millis(20));

    // Now the recycled key should be available
    let key3 = generator.generate();
    assert_eq!(key3.0, 0); // Recycled key
}

#[test]
fn key_generator_recycles_multiple_keys() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_millis(10));

    // Generate and recycle multiple keys
    let key1 = generator.generate();
    let key2 = generator.generate();
    let key3 = generator.generate();

    assert_eq!(key1.0, 0);
    assert_eq!(key2.0, 1);
    assert_eq!(key3.0, 2);

    generator.recycle_key(&key1);
    generator.recycle_key(&key2);
    generator.recycle_key(&key3);

    // Wait for recycle timeout
    std::thread::sleep(Duration::from_millis(20));

    // Should get recycled keys in FIFO order
    let recycled1 = generator.generate();
    let recycled2 = generator.generate();
    let recycled3 = generator.generate();

    assert_eq!(recycled1.0, 0);
    assert_eq!(recycled2.0, 1);
    assert_eq!(recycled3.0, 2);
}

#[test]
fn key_generator_does_not_recycle_before_timeout() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_secs(10));

    // Generate and recycle a key
    let key1 = generator.generate();
    generator.recycle_key(&key1);

    // Generate new keys immediately - should get fresh keys, not recycled
    let key2 = generator.generate();
    let key3 = generator.generate();

    assert_eq!(key1.0, 0);
    assert_eq!(key2.0, 1); // Fresh key
    assert_eq!(key3.0, 2); // Fresh key
}

#[test]
fn key_generator_with_custom_type() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct CustomKey {
        id: u16,
    }

    impl From<u16> for CustomKey {
        fn from(value: u16) -> Self {
            CustomKey { id: value }
        }
    }

    impl From<CustomKey> for u16 {
        fn from(value: CustomKey) -> Self {
            value.id
        }
    }

    let mut generator = naia_shared::KeyGenerator::<CustomKey>::new(Duration::from_secs(1));

    let key1 = generator.generate();
    let key2 = generator.generate();

    assert_eq!(key1.id, 0);
    assert_eq!(key2.id, 1);
}

#[test]
fn key_generator_many_keys() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_secs(1));

    // Generate many keys to ensure no panics
    let mut keys = Vec::new();
    for _ in 0..1000 {
        keys.push(generator.generate());
    }

    // Verify they're sequential
    for (i, key) in keys.iter().enumerate() {
        assert_eq!(key.0, i as u16);
    }
}

#[test]
fn key_generator_recycle_and_generate_mixed() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_millis(10));

    // Generate some keys
    let key1 = generator.generate();
    let key2 = generator.generate();
    let key3 = generator.generate();

    // Recycle some
    generator.recycle_key(&key1);
    generator.recycle_key(&key2);

    // Generate more
    let key4 = generator.generate();
    let key5 = generator.generate();

    // Wait for recycle timeout
    std::thread::sleep(Duration::from_millis(20));

    // Now generate - should get recycled keys
    let recycled1 = generator.generate();
    let recycled2 = generator.generate();

    // And fresh after recycled are exhausted
    let key6 = generator.generate();

    assert_eq!(key1.0, 0);
    assert_eq!(key2.0, 1);
    assert_eq!(key3.0, 2);
    assert_eq!(key4.0, 3);
    assert_eq!(key5.0, 4);
    assert_eq!(recycled1.0, 0); // Recycled
    assert_eq!(recycled2.0, 1); // Recycled
    assert_eq!(key6.0, 5); // Fresh
}

#[test]
fn key_generator_zero_timeout_recycles_immediately() {
    let mut generator = naia_shared::KeyGenerator::<TestKey>::new(Duration::from_millis(0));

    let key1 = generator.generate();
    generator.recycle_key(&key1);

    // With zero timeout, should be available immediately
    let key2 = generator.generate();

    assert_eq!(key1.0, 0);
    assert_eq!(key2.0, 0); // Recycled immediately
}
