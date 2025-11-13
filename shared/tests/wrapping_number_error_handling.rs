/// Tests for wrapping number error handling
/// Covers the panic removal from wrapping_number.rs utility module

use naia_shared::{try_wrapping_diff, wrapping_diff, WrappingNumberError};

#[test]
fn try_wrapping_diff_simple_cases() {
    // Simple forward difference
    assert_eq!(try_wrapping_diff(1, 2).unwrap(), 1);
    assert_eq!(try_wrapping_diff(10, 12).unwrap(), 2);

    // Simple backward difference
    assert_eq!(try_wrapping_diff(2, 1).unwrap(), -1);
    assert_eq!(try_wrapping_diff(12, 10).unwrap(), -2);

    // Same number
    assert_eq!(try_wrapping_diff(5, 5).unwrap(), 0);
}

#[test]
fn try_wrapping_diff_wrap_around() {
    // Wrapping from max to 0
    assert_eq!(try_wrapping_diff(65535, 0).unwrap(), 1);
    assert_eq!(try_wrapping_diff(65535, 1).unwrap(), 2);

    // Wrapping from 0 to max
    assert_eq!(try_wrapping_diff(0, 65535).unwrap(), -1);
    assert_eq!(try_wrapping_diff(1, 65535).unwrap(), -2);
}

#[test]
fn try_wrapping_diff_medium_wrap() {
    let diff: u16 = u16::MAX / 2;

    // Forward wrap
    let result = try_wrapping_diff(0, diff).unwrap();
    assert_eq!(i32::from(result), i32::from(diff));

    // Backward wrap
    let result = try_wrapping_diff(diff, 0).unwrap();
    assert_eq!(i32::from(result), -i32::from(diff));
}

#[test]
fn try_wrapping_diff_large_values() {
    // Test with large u16 values
    assert_eq!(try_wrapping_diff(65000, 65100).unwrap(), 100);
    assert_eq!(try_wrapping_diff(65100, 65000).unwrap(), -100);

    // Wrapping across boundary - calculate what it should actually be
    // From 65500 to 100: 65535-65500 = 35, then +1 to 0, then +100 = 136
    let result1 = try_wrapping_diff(65500, 100).unwrap();
    assert!(result1 > 0); // Should be positive (forward in time)

    let result2 = try_wrapping_diff(100, 65500).unwrap();
    assert!(result2 < 0); // Should be negative (backward in time)
}

#[test]
fn wrapping_diff_backward_compatible() {
    // All the original documented examples should work
    assert_eq!(wrapping_diff(1, 2), 1);
    assert_eq!(wrapping_diff(2, 1), -1);
    assert_eq!(wrapping_diff(65535, 0), 1);
    assert_eq!(wrapping_diff(0, 65535), -1);
}

#[test]
fn wrapping_diff_matches_try_version() {
    // For all valid inputs, wrapping_diff should equal try_wrapping_diff
    let test_values = vec![0, 1, 100, 1000, 10000, 32767, 32768, 50000, 65534, 65535];

    for &a in &test_values {
        for &b in &test_values {
            let result_try = try_wrapping_diff(a, b).unwrap();
            let result_regular = wrapping_diff(a, b);
            assert_eq!(
                result_try, result_regular,
                "Mismatch for wrapping_diff({}, {})",
                a, b
            );
        }
    }
}

#[test]
fn try_wrapping_diff_all_u16_combinations_sample() {
    // Test a sample of u16 combinations to ensure no panics
    // (Testing all 2^32 combinations would be too slow, so we sample)
    let step = 1000;
    for a in (0..=u16::MAX).step_by(step) {
        for b in (0..=u16::MAX).step_by(step) {
            // Should not panic or return error for any valid u16 input
            let result = try_wrapping_diff(a, b);
            assert!(
                result.is_ok(),
                "try_wrapping_diff({}, {}) should succeed but got error: {:?}",
                a,
                b,
                result
            );
        }
    }
}

#[test]
fn wrapping_diff_symmetry() {
    // wrapping_diff(a, b) should equal -wrapping_diff(b, a)
    // BUT: we need to handle the special case where the result is i16::MIN
    // because -i16::MIN overflows
    let test_values = vec![0, 1, 100, 1000, 16383, 16384, 32766, 32767, 49151, 49152];

    for &a in &test_values {
        for &b in &test_values {
            let forward = wrapping_diff(a, b);
            let backward = wrapping_diff(b, a);

            // Handle the i16::MIN special case
            if forward == i16::MIN {
                assert_eq!(backward, i16::MIN, "Special case: both should be MIN");
            } else {
                assert_eq!(
                    forward,
                    -backward,
                    "Symmetry broken for wrapping_diff({}, {})",
                    a,
                    b
                );
            }
        }
    }
}

#[test]
fn wrapping_diff_transitivity() {
    // If a -> b = x and b -> c = y, then a -> c should be related to x + y
    // (accounting for wrapping)
    let a: u16 = 100;
    let b: u16 = 200;
    let c: u16 = 300;

    let ab = wrapping_diff(a, b);
    let bc = wrapping_diff(b, c);
    let ac = wrapping_diff(a, c);

    // For sequential values without wrapping: ac should equal ab + bc
    assert_eq!(i32::from(ac), i32::from(ab) + i32::from(bc));
}

#[test]
fn wrapping_diff_edge_cases() {
    // Zero to zero
    assert_eq!(wrapping_diff(0, 0), 0);

    // Max to max
    assert_eq!(wrapping_diff(u16::MAX, u16::MAX), 0);

    // Zero to max
    assert_eq!(wrapping_diff(0, u16::MAX), -1);

    // Max to zero
    assert_eq!(wrapping_diff(u16::MAX, 0), 1);

    // Mid to mid
    let mid = u16::MAX / 2;
    assert_eq!(wrapping_diff(mid, mid), 0);
}

#[test]
fn error_display_format() {
    // Create an error manually (though it should never occur in practice)
    let error = WrappingNumberError::IntegerOverflow { a: 100, b: 200 };
    let error_string = format!("{}", error);

    assert!(error_string.contains("100"));
    assert!(error_string.contains("200"));
    assert!(error_string.contains("overflow"));
}

#[test]
fn wrapping_diff_consistency_with_existing_tests() {
    // These are from the existing tests in wrapping_number.rs
    // Ensure they still pass with the new implementation

    // Simple
    let a: u16 = 10;
    let b: u16 = 12;
    assert_eq!(wrapping_diff(a, b), 2);

    // Simple backwards
    assert_eq!(wrapping_diff(b, a), -2);

    // Max wrap
    let a: u16 = u16::MAX;
    let b: u16 = a.wrapping_add(2);
    assert_eq!(wrapping_diff(a, b), 2);

    // Min wrap
    let a: u16 = 0;
    let b: u16 = a.wrapping_sub(2);
    assert_eq!(wrapping_diff(a, b), -2);

    // Max wrap backwards
    let a: u16 = u16::MAX;
    let b: u16 = a.wrapping_add(2);
    assert_eq!(wrapping_diff(b, a), -2);

    // Min wrap backwards
    let a: u16 = 0;
    let b: u16 = a.wrapping_sub(2);
    assert_eq!(wrapping_diff(b, a), 2);
}

#[test]
fn sequence_greater_than_and_less_than() {
    use naia_shared::{sequence_greater_than, sequence_less_than};

    // Basic comparisons
    assert!(sequence_greater_than(2, 1));
    assert!(!sequence_greater_than(1, 2));
    assert!(!sequence_greater_than(1, 1));

    assert!(sequence_less_than(1, 2));
    assert!(!sequence_less_than(2, 1));
    assert!(!sequence_less_than(1, 1));

    // Wrapping cases
    assert!(sequence_greater_than(1, 65535));
    assert!(sequence_less_than(65535, 1));
}
