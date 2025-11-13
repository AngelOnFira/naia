use thiserror::Error;

/// Errors that can occur during wrapping number operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WrappingNumberError {
    /// Integer overflow occurred during wrapping difference calculation.
    /// This should be mathematically impossible with valid u16 inputs.
    #[error("Integer overflow in wrapping_diff({a}, {b}) - this should not happen")]
    IntegerOverflow { a: u16, b: u16 },
}

/// Returns whether or not a wrapping number is greater than another
/// sequence_greater_than(2,1) will return true
/// sequence_greater_than(1,2) will return false
/// sequence_greater_than(1,1) will return false
pub fn sequence_greater_than(s1: u16, s2: u16) -> bool {
    ((s1 > s2) && (s1 - s2 <= 32768)) || ((s1 < s2) && (s2 - s1 > 32768))
}

/// Returns whether or not a wrapping number is greater than another
/// sequence_less_than(1,2) will return true
/// sequence_less_than(2,1) will return false
/// sequence_less_than(1,1) will return false
pub fn sequence_less_than(s1: u16, s2: u16) -> bool {
    sequence_greater_than(s2, s1)
}

/// Retrieves the wrapping difference between 2 u16 values.
/// Returns an error if an impossible integer overflow occurs.
///
/// # Examples
/// ```
/// # use naia_shared::try_wrapping_diff;
/// assert_eq!(try_wrapping_diff(1, 2).unwrap(), 1);
/// assert_eq!(try_wrapping_diff(2, 1).unwrap(), -1);
/// assert_eq!(try_wrapping_diff(65535, 0).unwrap(), 1);
/// assert_eq!(try_wrapping_diff(0, 65535).unwrap(), -1);
/// ```
pub fn try_wrapping_diff(a: u16, b: u16) -> Result<i16, WrappingNumberError> {
    const MAX: i32 = std::i16::MAX as i32;
    const MIN: i32 = std::i16::MIN as i32;
    const ADJUST: i32 = (std::u16::MAX as i32) + 1;

    let a_i32: i32 = i32::from(a);
    let b_i32: i32 = i32::from(b);

    let mut result = b_i32 - a_i32;
    if (MIN..=MAX).contains(&result) {
        Ok(result as i16)
    } else if b_i32 > a_i32 {
        result = b_i32 - (a_i32 + ADJUST);
        if (MIN..=MAX).contains(&result) {
            Ok(result as i16)
        } else {
            Err(WrappingNumberError::IntegerOverflow { a, b })
        }
    } else {
        result = (b_i32 + ADJUST) - a_i32;
        if (MIN..=MAX).contains(&result) {
            Ok(result as i16)
        } else {
            Err(WrappingNumberError::IntegerOverflow { a, b })
        }
    }
}

/// Retrieves the wrapping difference between 2 u16 values.
///
/// # Panics
///
/// Panics if an impossible integer overflow occurs (this should never happen with valid u16 inputs).
///
/// # Examples
/// ```
/// # use naia_shared::wrapping_diff;
/// assert_eq!(wrapping_diff(1, 2), 1);
/// assert_eq!(wrapping_diff(2, 1), -1);
/// assert_eq!(wrapping_diff(65535, 0), 1);
/// assert_eq!(wrapping_diff(0, 65535), -1);
/// ```
pub fn wrapping_diff(a: u16, b: u16) -> i16 {
    try_wrapping_diff(a, b).expect("integer overflow in wrapping_diff - this should not happen")
}

#[cfg(test)]
mod sequence_compare_tests {
    use super::{sequence_greater_than, sequence_less_than};

    #[test]
    fn greater_is_greater() {
        assert!(sequence_greater_than(2, 1));
    }

    #[test]
    fn greater_is_not_equal() {
        assert!(!sequence_greater_than(2, 2));
    }

    #[test]
    fn greater_is_not_less() {
        assert!(!sequence_greater_than(1, 2));
    }

    #[test]
    fn less_is_less() {
        assert!(sequence_less_than(1, 2));
    }

    #[test]
    fn less_is_not_equal() {
        assert!(!sequence_less_than(2, 2));
    }

    #[test]
    fn less_is_not_greater() {
        assert!(!sequence_less_than(2, 1));
    }
}

#[cfg(test)]
mod wrapping_diff_tests {
    use super::wrapping_diff;

    #[test]
    fn simple() {
        let a: u16 = 10;
        let b: u16 = 12;

        let result = wrapping_diff(a, b);

        assert_eq!(result, 2);
    }

    #[test]
    fn simple_backwards() {
        let a: u16 = 10;
        let b: u16 = 12;

        let result = wrapping_diff(b, a);

        assert_eq!(result, -2);
    }

    #[test]
    fn max_wrap() {
        let a: u16 = std::u16::MAX;
        let b: u16 = a.wrapping_add(2);

        let result = wrapping_diff(a, b);

        assert_eq!(result, 2);
    }

    #[test]
    fn min_wrap() {
        let a: u16 = 0;
        let b: u16 = a.wrapping_sub(2);

        let result = wrapping_diff(a, b);

        assert_eq!(result, -2);
    }

    #[test]
    fn max_wrap_backwards() {
        let a: u16 = std::u16::MAX;
        let b: u16 = a.wrapping_add(2);

        let result = wrapping_diff(b, a);

        assert_eq!(result, -2);
    }

    #[test]
    fn min_wrap_backwards() {
        let a: u16 = 0;
        let b: u16 = a.wrapping_sub(2);

        let result = wrapping_diff(b, a);

        assert_eq!(result, 2);
    }

    #[test]
    fn medium_min_wrap() {
        let diff: u16 = std::u16::MAX / 2;
        let a: u16 = 0;
        let b: u16 = a.wrapping_sub(diff);

        let result = i32::from(wrapping_diff(a, b));

        assert_eq!(result, -i32::from(diff));
    }

    #[test]
    fn medium_min_wrap_backwards() {
        let diff: u16 = std::u16::MAX / 2;
        let a: u16 = 0;
        let b: u16 = a.wrapping_sub(diff);

        let result = i32::from(wrapping_diff(b, a));

        assert_eq!(result, i32::from(diff));
    }

    #[test]
    fn medium_max_wrap() {
        let diff: u16 = std::u16::MAX / 2;
        let a: u16 = std::u16::MAX;
        let b: u16 = a.wrapping_add(diff);

        let result = i32::from(wrapping_diff(a, b));

        assert_eq!(result, i32::from(diff));
    }

    #[test]
    fn medium_max_wrap_backwards() {
        let diff: u16 = std::u16::MAX / 2;
        let a: u16 = std::u16::MAX;
        let b: u16 = a.wrapping_add(diff);

        let result = i32::from(wrapping_diff(b, a));

        assert_eq!(result, -i32::from(diff));
    }
}
