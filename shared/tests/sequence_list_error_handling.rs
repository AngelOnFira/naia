/// Tests for SequenceList error handling
/// Covers the panic removal from sequence_list.rs utility module

use naia_shared::{SequenceError, SequenceList};

#[derive(Debug, PartialEq)]
struct TestItem {
    value: i32,
}

#[test]
fn try_insert_scan_from_back_success() {
    let mut list = SequenceList::new();

    // Should successfully insert unique IDs
    assert!(list.try_insert_scan_from_back(100, TestItem { value: 1 }).is_ok());
    assert!(list.try_insert_scan_from_back(200, TestItem { value: 2 }).is_ok());
    assert!(list.try_insert_scan_from_back(150, TestItem { value: 3 }).is_ok());
}

#[test]
fn try_insert_scan_from_back_duplicate_error() {
    let mut list = SequenceList::new();

    // Insert first item
    list.try_insert_scan_from_back(100, TestItem { value: 1 }).unwrap();

    // Try to insert duplicate ID
    let result = list.try_insert_scan_from_back(100, TestItem { value: 2 });

    assert!(result.is_err());
    match result {
        Err(SequenceError::DuplicateId { id }) => {
            assert_eq!(id, 100);
        }
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[test]
fn try_insert_multiple_duplicates() {
    let mut list = SequenceList::new();

    // Insert multiple items
    list.try_insert_scan_from_back(100, TestItem { value: 1 }).unwrap();
    list.try_insert_scan_from_back(200, TestItem { value: 2 }).unwrap();
    list.try_insert_scan_from_back(300, TestItem { value: 3 }).unwrap();

    // Try to insert duplicates for each
    assert!(matches!(
        list.try_insert_scan_from_back(100, TestItem { value: 4 }),
        Err(SequenceError::DuplicateId { id: 100 })
    ));
    assert!(matches!(
        list.try_insert_scan_from_back(200, TestItem { value: 5 }),
        Err(SequenceError::DuplicateId { id: 200 })
    ));
    assert!(matches!(
        list.try_insert_scan_from_back(300, TestItem { value: 6 }),
        Err(SequenceError::DuplicateId { id: 300 })
    ));
}

#[test]
#[should_panic(expected = "duplicates are not allowed")]
fn insert_scan_from_back_panics_on_duplicate() {
    let mut list = SequenceList::new();

    // Insert first item
    list.insert_scan_from_back(100, TestItem { value: 1 });

    // This should panic
    list.insert_scan_from_back(100, TestItem { value: 2 });
}

#[test]
fn insert_scan_from_back_backward_compatible() {
    let mut list = SequenceList::new();

    // Should work exactly like before for unique IDs
    list.insert_scan_from_back(100, TestItem { value: 1 });
    list.insert_scan_from_back(200, TestItem { value: 2 });
    list.insert_scan_from_back(150, TestItem { value: 3 });

    // Verify items are present
    assert!(list.contains_scan_from_back(&100));
    assert!(list.contains_scan_from_back(&200));
    assert!(list.contains_scan_from_back(&150));
}

#[test]
fn contains_scan_from_back_works_correctly() {
    let mut list = SequenceList::new();

    list.insert_scan_from_back(100, TestItem { value: 1 });
    list.insert_scan_from_back(200, TestItem { value: 2 });
    list.insert_scan_from_back(150, TestItem { value: 3 });

    // Should find existing items
    assert!(list.contains_scan_from_back(&100));
    assert!(list.contains_scan_from_back(&200));
    assert!(list.contains_scan_from_back(&150));

    // Should not find non-existent items
    assert!(!list.contains_scan_from_back(&50));
    assert!(!list.contains_scan_from_back(&250));
}

#[test]
fn get_mut_scan_from_back_works_correctly() {
    let mut list = SequenceList::new();

    list.insert_scan_from_back(100, TestItem { value: 1 });
    list.insert_scan_from_back(200, TestItem { value: 2 });
    list.insert_scan_from_back(150, TestItem { value: 3 });

    // Should find and allow mutation
    if let Some(item) = list.get_mut_scan_from_back(&100) {
        assert_eq!(item.value, 1);
        item.value = 42;
    }

    // Verify mutation worked
    if let Some(item) = list.get_mut_scan_from_back(&100) {
        assert_eq!(item.value, 42);
    }

    // Should return None for non-existent items
    assert!(list.get_mut_scan_from_back(&999).is_none());
}

#[test]
fn remove_scan_from_front_works_correctly() {
    let mut list = SequenceList::new();

    list.insert_scan_from_back(100, TestItem { value: 1 });
    list.insert_scan_from_back(200, TestItem { value: 2 });
    list.insert_scan_from_back(150, TestItem { value: 3 });

    // Should successfully remove existing items
    let removed = list.remove_scan_from_front(&100);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().value, 1);

    // Should not be present anymore
    assert!(!list.contains_scan_from_back(&100));

    // Should return None for non-existent items
    assert!(list.remove_scan_from_front(&999).is_none());
}

#[test]
fn sequence_operations_with_wrapping_numbers() {
    let mut list = SequenceList::new();

    // Test with wrapping u16 values
    list.insert_scan_from_back(65530, TestItem { value: 1 });
    list.insert_scan_from_back(65535, TestItem { value: 2 });
    list.insert_scan_from_back(0, TestItem { value: 3 });
    list.insert_scan_from_back(5, TestItem { value: 4 });

    assert!(list.contains_scan_from_back(&65530));
    assert!(list.contains_scan_from_back(&65535));
    assert!(list.contains_scan_from_back(&0));
    assert!(list.contains_scan_from_back(&5));
}

#[test]
fn error_display_format() {
    let error = SequenceError::DuplicateId { id: 12345 };
    let error_string = format!("{}", error);

    assert!(error_string.contains("12345"));
    assert!(error_string.contains("Duplicate"));
}

#[test]
fn pop_front_and_front_operations() {
    let mut list = SequenceList::new();

    list.insert_scan_from_back(100, TestItem { value: 1 });
    list.insert_scan_from_back(200, TestItem { value: 2 });

    // Front should show first item
    let front = list.front();
    assert!(front.is_some());

    // Pop front should remove first item
    let (_id, item) = list.pop_front();
    assert_eq!(item.value, 1);

    // Now front should be the second item
    let front = list.front();
    assert!(front.is_some());
    let (front_id, _) = front.unwrap();
    assert_eq!(*front_id, 200);
}
