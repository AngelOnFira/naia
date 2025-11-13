use crate::sequence_less_than;
use thiserror::Error;

/// Errors that can occur during SequenceList operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SequenceError {
    /// Attempted to insert a duplicate ID into the sequence list
    #[error("Duplicate sequence ID {id} not allowed in SequenceList")]
    DuplicateId { id: u16 },
}

pub struct SequenceList<T> {
    list: Vec<(u16, T)>,
}

impl<T> SequenceList<T> {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn front(&self) -> Option<&(u16, T)> {
        self.list.get(0)
    }

    pub fn pop_front(&mut self) -> (u16, T) {
        self.list.remove(0)
    }

    pub fn contains_scan_from_back(&self, id: &u16) -> bool {
        let mut index = self.list.len();

        loop {
            if index == 0 {
                // made it all the way through
                return false;
            }

            index -= 1;

            // SAFETY: index is always < list.len() because we checked above
            let (old_id, _) = unsafe { self.list.get_unchecked(index) };
            if *old_id == *id {
                return true;
            }
            if sequence_less_than(*old_id, *id) {
                return false;
            }
        }
    }

    pub fn get_mut_scan_from_back<'a>(&'a mut self, id: &u16) -> Option<&'a mut T> {
        let mut index = self.list.len();

        loop {
            if index == 0 {
                // made it all the way through
                return None;
            }

            index -= 1;

            {
                // SAFETY: index is always < list.len() because we checked above
                let (old_id, _) = unsafe { self.list.get_unchecked(index) };
                if *old_id == *id {
                    break;
                }
                if sequence_less_than(*old_id, *id) {
                    return None;
                }
            }
        }

        // SAFETY: We broke out of the loop, so index is still < list.len()
        let (_, item) = unsafe { self.list.get_unchecked_mut(index) };
        Some(item)
    }

    /// Attempts to insert an item with the given ID, scanning from the back.
    /// Returns an error if the ID already exists.
    pub fn try_insert_scan_from_back(&mut self, id: u16, item: T) -> Result<(), SequenceError> {
        let mut index = self.list.len();

        loop {
            if index == 0 {
                // made it all the way through, insert at front and be done
                self.list.insert(index, (id, item));
                return Ok(());
            }

            index -= 1;

            // SAFETY: index is always < list.len() because we checked above
            let (old_id, _) = unsafe { self.list.get_unchecked(index) };
            if *old_id == id {
                return Err(SequenceError::DuplicateId { id });
            }
            if sequence_less_than(*old_id, id) {
                self.list.insert(index + 1, (id, item));
                return Ok(());
            }
        }
    }

    /// Inserts an item with the given ID, scanning from the back.
    ///
    /// # Panics
    ///
    /// Panics if a duplicate ID already exists in the list.
    pub fn insert_scan_from_back(&mut self, id: u16, item: T) {
        self.try_insert_scan_from_back(id, item)
            .expect("duplicates are not allowed in SequenceList")
    }

    pub fn remove_scan_from_front(&mut self, id: &u16) -> Option<T> {
        let mut index = 0;
        let mut remove = false;

        loop {
            if index >= self.list.len() {
                return None;
            }

            // SAFETY: index is always < list.len() because we checked above
            let (old_id, _) = unsafe { self.list.get_unchecked(index) };
            if *old_id == *id {
                remove = true;
            }

            if remove {
                return Some(self.list.remove(index).1);
            }

            index += 1;
        }
    }
}
