use naia_shared::{GameInstant, GAME_TIME_LIMIT};

// GAME_TIME_MAX is not exported, but we can calculate it
const GAME_TIME_MAX: u32 = GAME_TIME_LIMIT - 1;

// Helper function to create GameInstant for testing
// We use the public API by creating from an Instant and then manipulating
fn create_test_instant(millis: u32) -> GameInstant {
    use naia_socket_shared::Instant;
    let start = Instant::now();
    let instant = GameInstant::new(&start);
    // Use sub_millis to get to 0, then add_millis to get to desired value
    let at_zero = instant.sub_millis(instant.as_millis());
    at_zero.add_millis(millis)
}

#[test]
fn test_try_offset_from_basic() {
    let a = create_test_instant(10);
    let b = create_test_instant(12);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(2));
}

#[test]
fn test_try_offset_from_backwards() {
    let a = create_test_instant(10);
    let b = create_test_instant(12);

    let result = b.try_offset_from(&a);
    assert_eq!(result, Some(-2));
}

#[test]
fn test_try_offset_from_same() {
    let a = create_test_instant(100);
    let b = create_test_instant(100);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(0));
}

#[test]
fn test_try_offset_from_wrap_forward() {
    let a = create_test_instant(GAME_TIME_MAX);
    let b = a.add_millis(5);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(5));
}

#[test]
fn test_try_offset_from_wrap_backward() {
    let a = create_test_instant(0);
    let b = a.sub_millis(5);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(-5));
}

#[test]
fn test_try_offset_from_large_forward() {
    let a = create_test_instant(1000);
    let b = create_test_instant(1000 + 1_000_000);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(1_000_000));
}

#[test]
fn test_try_offset_from_large_backward() {
    let a = create_test_instant(1000 + 1_000_000);
    let b = create_test_instant(1000);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(-1_000_000));
}

#[test]
fn test_try_offset_from_at_boundaries() {
    // Test at max valid offset
    let diff = GAME_TIME_LIMIT / 2 - 1;
    let a = create_test_instant(0);
    let b = a.add_millis(diff);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(diff as i32));
}

#[test]
fn test_try_offset_from_wrap_near_boundary() {
    let diff = GAME_TIME_LIMIT / 2;
    let a = create_test_instant(0);
    let b = a.sub_millis(diff);

    let result = a.try_offset_from(&b);
    assert_eq!(result, Some(-(diff as i32)));
}

#[test]
fn test_is_more_than_basic() {
    let earlier = create_test_instant(10);
    let later = create_test_instant(20);

    assert!(later.is_more_than(&earlier));
    assert!(!earlier.is_more_than(&later));
}

#[test]
fn test_is_more_than_same() {
    let a = create_test_instant(100);
    let b = create_test_instant(100);

    assert!(!a.is_more_than(&b));
    assert!(!b.is_more_than(&a));
}

#[test]
fn test_is_more_than_with_wrap() {
    let a = create_test_instant(GAME_TIME_MAX);
    let b = a.add_millis(10);

    assert!(b.is_more_than(&a));
    assert!(!a.is_more_than(&b));
}

#[test]
fn test_add_millis_no_wrap() {
    let a = create_test_instant(100);
    let b = a.add_millis(50);

    assert_eq!(b.as_millis(), 150);
}

#[test]
fn test_add_millis_with_wrap() {
    let a = create_test_instant(GAME_TIME_MAX);
    let b = a.add_millis(10);

    // Should wrap around
    assert_eq!(b.as_millis(), 9);
}

#[test]
fn test_sub_millis_no_wrap() {
    let a = create_test_instant(100);
    let b = a.sub_millis(50);

    assert_eq!(b.as_millis(), 50);
}

#[test]
fn test_sub_millis_with_wrap() {
    let a = create_test_instant(10);
    let b = a.sub_millis(20);

    // Should wrap to near the max
    assert_eq!(b.as_millis(), GAME_TIME_LIMIT - 10);
}

#[test]
fn test_add_signed_millis_positive() {
    let a = create_test_instant(100);
    let b = a.add_signed_millis(50);

    assert_eq!(b.as_millis(), 150);
}

#[test]
fn test_add_signed_millis_negative() {
    let a = create_test_instant(100);
    let b = a.add_signed_millis(-50);

    assert_eq!(b.as_millis(), 50);
}

#[test]
fn test_add_signed_millis_zero() {
    let a = create_test_instant(100);
    let b = a.add_signed_millis(0);

    assert_eq!(b.as_millis(), 100);
}

#[test]
fn test_time_since_forward() {
    let earlier = create_test_instant(100);
    let later = create_test_instant(150);

    let duration = later.time_since(&earlier);
    assert_eq!(duration.as_millis(), 50);
}

#[test]
fn test_time_since_with_wrap() {
    let earlier = create_test_instant(GAME_TIME_MAX - 10);
    let later = create_test_instant(10);

    let duration = later.time_since(&earlier);
    assert_eq!(duration.as_millis(), 20);
}

#[test]
fn test_time_since_same_instant() {
    let a = create_test_instant(100);

    let duration = a.time_since(&a);
    assert_eq!(duration.as_millis(), 0);
}

// Note: Timestamp tests are not included here because the backends module
// is not publicly exported. The TimeError type is defined and available for
// library users, but testing it directly from the test file would require
// making backends public, which is not desirable.
//
// The Timestamp::try_now() method is tested indirectly through integration
// tests and actual usage in the codebase.
