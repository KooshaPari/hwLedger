//! In-memory storage implementations for cache tiers.
//!
//! L1: LRU cache using bounded capacity
//! L2: Concurrent DashMap with TTL eviction

use crate::domain::entities::CacheEntry;
use crate::domain::ports::outbound::EntryStore;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::RwLock;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// L1 tier - Bounded LRU cache for hot data.
pub struct L1Tier<K, V>
where
    K: Hash + Eq,
    V: Clone,
{
    cache: RwLock<LruCache<K, CacheEntry<V>>>,
}

impl<K, V> L1Tier<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity.max(1)).unwrap(),
            )),
        }
    }

    pub fn get(&self, key: &K) -> Option<CacheEntry<V>> {
        self.cache.write().get(key).cloned()
    }

    pub fn put(&self, key: K, entry: CacheEntry<V>) {
        self.cache.write().put(key, entry);
    }

    pub fn pop(&self, key: &K) -> Option<CacheEntry<V>> {
        self.cache.write().pop(key)
    }

    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    pub fn clear(&self) {
        self.cache.write().clear();
    }

    pub fn contains(&self, key: &K) -> bool {
        self.cache.read().contains(key)
    }
}

/// L2 tier - Concurrent DashMap for warm data with TTL.
pub struct L2Tier<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    map: DashMap<K, CacheEntry<V>>,
}

impl<K, V> L2Tier<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn get(&self, key: &K) -> Option<CacheEntry<V>> {
        self.map.get(key).map(|r| r.clone())
    }

    pub fn insert(&self, key: K, entry: CacheEntry<V>) {
        self.map.insert(key, entry);
    }

    pub fn remove(&self, key: &K) -> Option<CacheEntry<V>> {
        self.map.remove(key).map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn clear(&self) {
        self.map.clear();
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}

impl<K, V> Default for L2Tier<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Combined in-memory entry store with two tiers.
pub struct InMemoryEntryStore<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    l1: Arc<L1Tier<K, V>>,
    l2: Arc<L2Tier<K, V>>,
}

impl<K, V> InMemoryEntryStore<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(l1_capacity: usize) -> Self {
        Self {
            l1: Arc::new(L1Tier::new(l1_capacity)),
            l2: Arc::new(L2Tier::new()),
        }
    }

    pub fn l1(&self) -> &L1Tier<K, V> {
        &self.l1
    }

    pub fn l2(&self) -> &L2Tier<K, V> {
        &self.l2
    }
}

impl<K, V> EntryStore<K, V> for InMemoryEntryStore<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn get(&self, key: &K) -> Option<CacheEntry<V>> {
        // Check L1 first
        if let Some(entry) = self.l1.get(key) {
            return Some(entry);
        }
        // Fallback to L2
        self.l2.get(key)
    }

    fn insert(&self, key: K, entry: CacheEntry<V>) {
        self.l1.put(key.clone(), entry.clone());
        self.l2.insert(key, entry);
    }

    fn remove(&self, key: &K) {
        self.l1.pop(key);
        self.l2.remove(key);
    }

    fn clear(&self) {
        self.l1.clear();
        self.l2.clear();
    }

    fn contains(&self, key: &K) -> Option<bool> {
        Some(self.l1.contains(key) || self.l2.contains_key(key))
    }
}

/// In-memory tier enum for selection.
pub enum InMemoryTier {
    L1Only,
    L2Only,
    Both,
}
