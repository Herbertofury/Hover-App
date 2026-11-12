use std::collections::VecDeque;
use std::hash::Hash;

pub trait ByteSized {
    fn byte_size(&self) -> usize;
}

#[derive(Debug)]
pub struct HardCappedLru<K, V> {
    max_bytes: usize,
    current_bytes: usize,
    entries: VecDeque<(K, V)>,
}

impl<K, V> HardCappedLru<K, V>
where
    K: Eq + Hash,
    V: ByteSized,
{
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            current_bytes: 0,
            entries: VecDeque::new(),
        }
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == key)?;
        if index != self.entries.len() - 1 {
            let entry = self.entries.remove(index)?;
            self.entries.push_back(entry);
        }
        self.entries.back().map(|(_, value)| value)
    }

    pub fn put(&mut self, key: K, value: V) -> bool {
        let value_bytes = value.byte_size();
        if value_bytes > self.max_bytes {
            return false;
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|(existing, _)| existing == &key)
            && let Some((_, old)) = self.entries.remove(index)
        {
            self.current_bytes = self.current_bytes.saturating_sub(old.byte_size());
        }
        self.current_bytes = self.current_bytes.saturating_add(value_bytes);
        self.entries.push_back((key, value));
        self.evict_to_cap();
        true
    }

    fn evict_to_cap(&mut self) {
        while self.current_bytes > self.max_bytes {
            if let Some((_, value)) = self.entries.pop_front() {
                self.current_bytes = self.current_bytes.saturating_sub(value.byte_size());
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Blob(usize);

    impl ByteSized for Blob {
        fn byte_size(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn cache_evicts_to_hard_cap() {
        let mut cache = HardCappedLru::new(10);
        assert!(cache.put("a", Blob(4)));
        assert!(cache.put("b", Blob(4)));
        assert!(cache.put("c", Blob(4)));
        assert!(cache.current_bytes() <= 10);
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&"a").is_none());
    }

    #[test]
    fn cache_rejects_item_larger_than_cap() {
        let mut cache = HardCappedLru::new(10);
        assert!(!cache.put("huge", Blob(11)));
        assert_eq!(cache.current_bytes(), 0);
    }
}
