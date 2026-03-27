use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct CacheEntry<T> {
    inserted_at: Instant,
    value: T,
}

#[derive(Debug)]
pub struct TimedCache<T> {
    ttl: Duration,
    entries: Mutex<HashMap<String, CacheEntry<T>>>,
}

impl<T: Clone> TimedCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let mut entries = self.entries.lock().expect("cache poisoned");
        if let Some(entry) = entries.get(key) {
            if entry.inserted_at.elapsed() <= self.ttl {
                return Some(entry.value.clone());
            }
        }
        entries.remove(key);
        None
    }

    pub fn insert(&self, key: String, value: T) {
        let mut entries = self.entries.lock().expect("cache poisoned");
        entries.insert(
            key,
            CacheEntry {
                inserted_at: Instant::now(),
                value,
            },
        );
    }
}
