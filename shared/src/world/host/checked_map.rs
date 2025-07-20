use std::collections::{HashMap, HashSet};
use std::hash::Hash;

// CheckedMap
pub struct CheckedMap<K: Eq + Hash, V> {
    pub inner: HashMap<K, V>,
}

impl<K: Eq + Hash, V> CheckedMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner.get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.inner.contains_key(&key) {
            panic!("Cannot insert and replace value for given key. Check first.")
        }

        self.inner.insert(key, value);
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if !self.inner.contains_key(key) {
            panic!("Cannot remove value for key with non-existent value. Check whether map contains key first.")
        }

        self.inner.remove(key)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<K, V> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

// CheckedSet
pub struct CheckedSet<K: Eq + Hash> {
    pub inner: HashSet<K>,
}

impl<K: Eq + Hash> CheckedSet<K> {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.inner.contains(key)
    }

    pub fn insert(&mut self, key: K) {
        if self.inner.contains(&key) {
            panic!("Cannot insert and replace given key. Check first.")
        }

        self.inner.insert(key);
    }

    pub fn remove(&mut self, key: &K) {
        if !self.inner.contains(key) {
            panic!("Cannot remove given non-existent key. Check first.")
        }

        self.inner.remove(key);
    }

    pub fn iter(&self) -> std::collections::hash_set::Iter<K> {
        self.inner.iter()
    }
}