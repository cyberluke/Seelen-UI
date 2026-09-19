use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

/// True lock-free map: a `scc::HashMap` alias whose entries hold an `Arc` to an
/// immutable value. Readers clone the `Arc` (or read through it) and never
/// mutate the map, so several `Arc` references to the same value share one
/// allocation.
pub struct SyncHashMap<K, V>(scc::HashMap<K, Arc<V>>);

/// `&mut` access to the stored value. During the synchronous writer phase of
/// scc the entry `Arc` is uniquely owned (all wrapper reads return owned
/// clones), so the exclusive reference is guaranteed.
#[inline]
fn as_mut<V>(arc: &mut Arc<V>) -> &mut V {
    Arc::get_mut(arc).expect("exclusive Arc reference during scc writer phase")
}

#[allow(dead_code)]
impl<K, V> SyncHashMap<K, V>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        Self(scc::HashMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(scc::HashMap::with_capacity(capacity))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn upsert(&self, key: K, value: V) -> Option<V> {
        self.0
            .upsert_sync(key, Arc::new(value))
            .and_then(Arc::into_inner)
    }

    #[inline]
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.0
            .remove_sync(key)
            .and_then(|(_, value)| Arc::into_inner(value))
    }

    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.0.contains_sync(key)
    }

    /// Read through the shared value (no map mutation on the read path).
    #[inline]
    pub fn get<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        self.0.update_sync(key, |_, value| f(as_mut(value)))
    }

    /// If key does not exist, it will be created with default value
    #[inline]
    pub fn get_or_default<Q, F, R>(&self, key: Q, f: F) -> R
    where
        V: Default,
        Q: Into<K>,
        F: FnOnce(&mut V) -> R,
    {
        let key = key.into();
        let mut f = Some(f);
        if let Some(out) = self
            .0
            .update_sync(&key, |_, value| f.take().unwrap()(as_mut(value)))
        {
            return out;
        }
        let mut value = V::default();
        let out = f.take().unwrap()(&mut value);
        self.0.upsert_sync(key, Arc::new(value));
        out
    }

    /// If key does not exist, it will be created using the provided constructor function
    #[inline]
    pub fn get_or_insert<Q, C, F, R>(&self, key: Q, constructor: C, f: F) -> R
    where
        Q: Into<K>,
        C: FnOnce() -> V,
        F: FnOnce(&mut V) -> R,
    {
        let key = key.into();
        let mut f = Some(f);
        if let Some(out) = self
            .0
            .update_sync(&key, |_, value| f.take().unwrap()(as_mut(value)))
        {
            return out;
        }
        let mut value = constructor();
        let out = f.take().unwrap()(&mut value);
        self.0.upsert_sync(key, Arc::new(value));
        out
    }

    /// Visit every entry while the map is stable; per-entry work should be
    /// cheap. For potentially blocking work prefer `key_snapshot` + `get`.
    #[inline]
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut((&K, &mut V)),
    {
        self.0.iter_mut_sync(|mut entry| {
            let key_ptr: *const K = entry.key();
            // SAFETY: the key pointer comes from the entry being visited and is
            // only read once; the mutable borrow of the value does not touch it.
            f((unsafe { &*key_ptr }, as_mut(&mut *entry)));
            true
        });
    }

    /// Snapshot the keys into a `Vec`, so later per-entry `get` calls never
    /// hold an iteration guard over the map.
    #[inline]
    pub fn key_snapshot(&self) -> Vec<K>
    where
        K: Clone,
    {
        let mut keys = Vec::with_capacity(self.0.len());
        self.0.iter_sync(|key, _| {
            keys.push(key.clone());
            true
        });
        keys
    }

    #[inline]
    pub fn retain<F>(&self, mut f: F)
    where
        F: FnMut((&K, &mut V)) -> bool,
    {
        self.0
            .retain_sync(move |key, value| f((key, as_mut(value))));
    }

    #[inline]
    pub fn clear(&self) {
        self.0.clear_sync();
    }

    #[inline]
    pub fn any<F>(&self, mut f: F) -> bool
    where
        F: FnMut((&K, &V)) -> bool,
    {
        self.0
            .any_sync(move |key, value| f((key, &**value)))
            .is_some()
    }

    #[inline]
    pub fn take(&self) -> HashMap<K, V>
    where
        K: Clone,
        V: Clone,
    {
        let mut out: HashMap<K, V> = HashMap::with_capacity(self.0.len());
        self.0.iter_sync(|key, value| {
            out.insert(key.clone(), (**value).clone());
            true
        });
        self.0.clear_sync();
        out
    }

    #[inline]
    pub fn replace(&self, value: HashMap<K, V>) {
        self.0.clear_sync();
        for (key, value) in value {
            self.0.upsert_sync(key, Arc::new(value));
        }
    }

    /// Run `f` over a plain snapshot of the map and write the result back.
    /// The maps stay small by construction, so the snapshot round-trip is cheap
    /// compared to holding a lock across arbitrary `f` work.
    #[inline]
    pub fn with_lock<F, R>(&self, f: F) -> R
    where
        K: Clone,
        V: Clone,
        F: FnOnce(&mut HashMap<K, V>) -> R,
    {
        let mut snapshot = self.take();
        let out = f(&mut snapshot);
        for (key, value) in snapshot {
            self.0.upsert_sync(key, Arc::new(value));
        }
        out
    }
}

#[allow(dead_code)]
impl<K, V> SyncHashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    #[inline]
    pub fn to_hash_map(&self) -> HashMap<K, V> {
        let mut out: HashMap<K, V> = HashMap::with_capacity(self.0.len());
        self.0.iter_sync(|key, value| {
            out.insert(key.clone(), (**value).clone());
            true
        });
        out
    }

    #[inline]
    pub fn keys(&self) -> Vec<K> {
        self.key_snapshot()
    }

    #[inline]
    pub fn values(&self) -> Vec<V> {
        let mut out = Vec::with_capacity(self.0.len());
        self.0.iter_sync(|_, value| {
            out.push((**value).clone());
            true
        });
        out
    }
}

impl<K, V> From<HashMap<K, V>> for SyncHashMap<K, V>
where
    K: Eq + Hash,
{
    fn from(value: HashMap<K, V>) -> Self {
        let map = Self(scc::HashMap::with_capacity(value.len()));
        for (key, value) in value {
            map.0.upsert_sync(key, Arc::new(value));
        }
        map
    }
}

impl<K, V> Default for SyncHashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_snapshot_returns_all_keys_once() {
        let map: SyncHashMap<u32, String> = SyncHashMap::new();
        map.upsert(1, "a".into());
        map.upsert(2, "b".into());
        let mut keys = map.key_snapshot();
        keys.sort();
        assert_eq!(keys, vec![1, 2]);
    }

    #[test]
    fn upsert_remove_and_len() {
        let map: SyncHashMap<u32, Vec<u8>> = SyncHashMap::new();
        assert_eq!(map.upsert(1, vec![1]), None);
        assert_eq!(map.upsert(1, vec![2]), Some(vec![1]));
        assert_eq!(map.remove(&1), Some(vec![2]));
        assert!(map.is_empty());
    }

    #[test]
    fn entries_share_arc_backing() {
        let map: SyncHashMap<u32, Vec<u8>> = SyncHashMap::new();
        map.upsert(1, vec![7]);
        // two reads of the same entry must see the same backing storage
        let first = map.get(&1, |v| v.as_ptr());
        let second = map.get(&1, |v| v.as_ptr());
        assert_eq!(first, second);
    }

    #[test]
    fn for_each_and_retain_mutate_entries() {
        let map: SyncHashMap<u32, Vec<u8>> =
            SyncHashMap::from(HashMap::from([(1, vec![1]), (2, vec![2])]));
        map.for_each(|(_, value)| value.push(9));
        assert_eq!(map.get(&1, |v| v.clone()), Some(vec![1, 9]));
        map.retain(|(key, _)| *key == 1);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn take_drains_into_plain_map() {
        let map: SyncHashMap<u32, u32> = SyncHashMap::from(HashMap::from([(1, 1), (2, 2)]));
        let plain = map.take();
        assert_eq!(plain.len(), 2);
        assert!(map.is_empty());
    }
}
