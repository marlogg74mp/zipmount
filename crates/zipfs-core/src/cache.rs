//! An LRU cache of decompressed contents, bounded in bytes.
//!
//! The budget is not decoration: without it the very first recursive `grep`,
//! or copying out a multi-gigabyte tree, would drag the whole archive into
//! memory.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rustc_hash::FxHashMap;

/// How much memory decompressed entries may hold.
pub const DEFAULT_BUDGET: usize = 512 * 1024 * 1024;

pub struct ContentCache {
    inner: Mutex<Inner>,
    budget: usize,
    hits: AtomicU64,
    misses: AtomicU64,
}

struct Inner {
    /// entry key -> (data, sequence number of the last access)
    map: FxHashMap<u32, (Arc<[u8]>, u64)>,
    /// access order: the smallest number is the next to evict
    order: BTreeMap<u64, u32>,
    used: usize,
    seq: u64,
}

impl ContentCache {
    pub fn new(budget: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                map: FxHashMap::default(),
                order: BTreeMap::new(),
                used: 0,
                seq: 0,
            }),
            budget,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn get(&self, key: u32) -> Option<Arc<[u8]>> {
        let mut inner = self.inner.lock().expect("cache poisoned by a panic");
        let Some((data, old_seq)) = inner.map.get(&key).cloned() else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        };

        // Move it up in the access order.
        inner.order.remove(&old_seq);
        inner.seq += 1;
        let new_seq = inner.seq;
        inner.order.insert(new_seq, key);
        inner.map.insert(key, (data.clone(), new_seq));

        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(data)
    }

    /// Puts an entry into the cache. Entries that are too large are not
    /// cached at all: one of them would otherwise evict everything else for
    /// the sake of a single read.
    pub fn insert(&self, key: u32, data: Arc<[u8]>) {
        if data.len() > self.budget / 2 {
            return;
        }

        let mut inner = self.inner.lock().expect("cache poisoned by a panic");
        if let Some((old, old_seq)) = inner.map.remove(&key) {
            inner.used -= old.len();
            inner.order.remove(&old_seq);
        }

        inner.seq += 1;
        let seq = inner.seq;
        inner.used += data.len();
        inner.order.insert(seq, key);
        inner.map.insert(key, (data, seq));

        while inner.used > self.budget {
            let Some((&oldest_seq, &oldest_key)) = inner.order.iter().next() else {
                break;
            };
            inner.order.remove(&oldest_seq);
            if let Some((evicted, _)) = inner.map.remove(&oldest_key) {
                inner.used -= evicted.len();
            }
        }
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.lock().expect("cache poisoned by a panic");
        CacheStats {
            used_bytes: inner.used,
            budget_bytes: self.budget,
            entries: inner.map.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}

impl Default for ContentCache {
    fn default() -> Self {
        Self::new(DEFAULT_BUDGET)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub used_bytes: usize,
    pub budget_bytes: usize,
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(n: usize) -> Arc<[u8]> {
        vec![0u8; n].into()
    }

    #[test]
    fn returns_what_was_stored() {
        let c = ContentCache::new(1000);
        c.insert(1, blob(100));
        assert_eq!(c.get(1).map(|d| d.len()), Some(100));
        assert!(c.get(2).is_none());
    }

    #[test]
    fn evicts_when_over_budget() {
        let c = ContentCache::new(300);
        c.insert(1, blob(100));
        c.insert(2, blob(100));
        c.insert(3, blob(100));
        c.insert(4, blob(100)); // over budget -> the oldest must go
        assert!(c.get(1).is_none(), "the oldest entry must be evicted");
        assert!(c.get(4).is_some());
        assert!(c.stats().used_bytes <= 300);
    }

    #[test]
    fn recent_access_protects_from_eviction() {
        let c = ContentCache::new(300);
        c.insert(1, blob(100));
        c.insert(2, blob(100));
        c.insert(3, blob(100));
        // Touch the first — now the second is the oldest.
        let _ = c.get(1);
        c.insert(4, blob(100));
        assert!(c.get(1).is_some(), "a recently used entry must survive");
        assert!(c.get(2).is_none());
    }

    #[test]
    fn oversized_entry_is_not_cached_at_all() {
        let c = ContentCache::new(1000);
        c.insert(1, blob(100));
        c.insert(2, blob(900)); // more than half the budget
        assert!(
            c.get(2).is_none(),
            "a large entry must not occupy the cache"
        );
        assert!(c.get(1).is_some(), "nor evict anything useful");
    }

    #[test]
    fn reinsert_does_not_double_count_bytes() {
        let c = ContentCache::new(1000);
        c.insert(1, blob(100));
        c.insert(1, blob(100));
        assert_eq!(c.stats().used_bytes, 100);
        assert_eq!(c.stats().entries, 1);
    }

    #[test]
    fn budget_holds_under_many_inserts() {
        let c = ContentCache::new(10_000);
        for i in 0..500u32 {
            c.insert(i, blob(100));
        }
        assert!(c.stats().used_bytes <= 10_000);
    }
}
