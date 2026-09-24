//! Cache of expanded 7z solid blocks.
//!
//! A layer separate from the per-file cache, because 7z has a different unit
//! of access: getting one file out means expanding its whole block. A block
//! here can run to gigabytes, so the eviction rule is single but important:
//! **the most recent block is never evicted**, even if it alone exceeds the
//! budget. Otherwise every read would expand the block again.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rustc_hash::FxHashMap;

/// By default up to 6 GB of expanded blocks are kept: enough for a typical
/// log archive to settle in memory entirely after the first pass.
pub const DEFAULT_BLOCK_BUDGET: usize = 6 * 1024 * 1024 * 1024;

/// An expanded block: the contents of each of its entries separately.
///
/// Keeping entries apart rather than as one piece pays twice: no
/// gigabyte-sized contiguous allocation is needed, and an open file holds on
/// to its own entry only, not the whole block.
pub struct DecodedBlock {
    entries: FxHashMap<u32, Arc<[u8]>>,
    bytes: usize,
}

impl DecodedBlock {
    pub fn new(entries: FxHashMap<u32, Arc<[u8]>>) -> Self {
        let bytes = entries.values().map(|d| d.len()).sum();
        Self { entries, bytes }
    }

    pub fn get(&self, entry: u32) -> Option<Arc<[u8]>> {
        self.entries.get(&entry).cloned()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

pub struct BlockCache {
    inner: Mutex<Inner>,
    budget: usize,
    hits: AtomicU64,
    misses: AtomicU64,
}

struct Inner {
    blocks: FxHashMap<usize, (Arc<DecodedBlock>, u64)>,
    order: BTreeMap<u64, usize>,
    used: usize,
    seq: u64,
}

impl BlockCache {
    pub fn new(budget: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                blocks: FxHashMap::default(),
                order: BTreeMap::new(),
                used: 0,
                seq: 0,
            }),
            budget,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn get(&self, block: usize) -> Option<Arc<DecodedBlock>> {
        let mut inner = self.inner.lock().expect("block cache poisoned by a panic");
        let Some((data, old_seq)) = inner.blocks.get(&block).cloned() else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        };

        inner.order.remove(&old_seq);
        inner.seq += 1;
        let new_seq = inner.seq;
        inner.order.insert(new_seq, block);
        inner.blocks.insert(block, (data.clone(), new_seq));

        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(data)
    }

    pub fn insert(&self, block: usize, data: Arc<DecodedBlock>) {
        let mut inner = self.inner.lock().expect("block cache poisoned by a panic");

        if let Some((old, old_seq)) = inner.blocks.remove(&block) {
            inner.used -= old.bytes();
            inner.order.remove(&old_seq);
        }

        inner.seq += 1;
        let seq = inner.seq;
        inner.used += data.bytes();
        inner.order.insert(seq, block);
        inner.blocks.insert(block, (data, seq));

        // Evict the oldest until the budget is met, but never touch the last
        // remaining block.
        while inner.used > self.budget && inner.blocks.len() > 1 {
            let Some((&oldest_seq, &oldest_block)) = inner.order.iter().next() else {
                break;
            };
            inner.order.remove(&oldest_seq);
            if let Some((evicted, _)) = inner.blocks.remove(&oldest_block) {
                inner.used -= evicted.bytes();
            }
        }
    }

    pub fn stats(&self) -> BlockCacheStats {
        let inner = self.inner.lock().expect("block cache poisoned by a panic");
        BlockCacheStats {
            used_bytes: inner.used,
            budget_bytes: self.budget,
            blocks: inner.blocks.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockCacheStats {
    pub used_bytes: usize,
    pub budget_bytes: usize,
    pub blocks: usize,
    pub hits: u64,
    pub misses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(entry_count: u32, each: usize) -> Arc<DecodedBlock> {
        let mut map = FxHashMap::default();
        for i in 0..entry_count {
            map.insert(i, vec![0u8; each].into());
        }
        Arc::new(DecodedBlock::new(map))
    }

    #[test]
    fn returns_entry_from_cached_block() {
        let c = BlockCache::new(1000);
        c.insert(0, block(3, 100));
        let b = c.get(0).expect("the block must be found");
        assert_eq!(b.get(1).map(|d| d.len()), Some(100));
        assert!(b.get(99).is_none());
    }

    #[test]
    fn evicts_oldest_when_over_budget() {
        let c = BlockCache::new(500);
        c.insert(0, block(2, 100)); // 200
        c.insert(1, block(2, 100)); // 200
        c.insert(2, block(2, 100)); // 200 -> over 500
        assert!(c.get(0).is_none(), "the oldest block must be evicted");
        assert!(c.get(2).is_some());
    }

    #[test]
    fn recent_use_protects_block() {
        let c = BlockCache::new(500);
        c.insert(0, block(2, 100));
        c.insert(1, block(2, 100));
        let _ = c.get(0);
        c.insert(2, block(2, 100));
        assert!(c.get(0).is_some(), "a recently used block must survive");
        assert!(c.get(1).is_none());
    }

    #[test]
    fn single_oversized_block_is_kept() {
        // The main difference from the per-file cache: a block larger than
        // the budget stays anyway, or every read would expand it again.
        let c = BlockCache::new(100);
        c.insert(0, block(1, 10_000));
        assert!(c.get(0).is_some(), "the only block must not be evicted");
        assert!(c.stats().used_bytes > c.stats().budget_bytes);
    }

    #[test]
    fn oversized_block_replaces_previous_one() {
        let c = BlockCache::new(100);
        c.insert(0, block(1, 10_000));
        c.insert(1, block(1, 10_000));
        assert_eq!(c.stats().blocks, 1, "blocks over budget must not pile up");
        assert!(
            c.get(1).is_some(),
            "the most recent one must be the one left"
        );
    }

    #[test]
    fn reinsert_does_not_double_count() {
        let c = BlockCache::new(10_000);
        c.insert(0, block(2, 100));
        c.insert(0, block(2, 100));
        assert_eq!(c.stats().used_bytes, 200);
        assert_eq!(c.stats().blocks, 1);
    }
}
