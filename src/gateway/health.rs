use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;

#[derive(Debug, Clone, Default)]
struct HealthEntry {
    successes: u64,
    failures: u64,
    consecutive_failures: u32,
    open_until: Option<Instant>,
    last_error: Option<String>,
    last_latency_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthSnapshot {
    pub provider: String,
    pub successes: u64,
    pub failures: u64,
    pub consecutive_failures: u32,
    pub circuit_open: bool,
    pub circuit_remaining_ms: Option<u128>,
    pub last_error: Option<String>,
    pub last_latency_ms: Option<u128>,
}

pub struct HealthStore {
    threshold: u32,
    open_duration: Duration,
    entries: Mutex<HashMap<String, HealthEntry>>,
}

impl HealthStore {
    pub fn new(threshold: u32, open_duration: Duration) -> Self {
        Self {
            threshold,
            open_duration,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_available(&self, provider: &str) -> bool {
        let mut entries = self.entries.lock().expect("health store poisoned");
        let entry = entries.entry(provider.to_string()).or_default();
        if let Some(until) = entry.open_until {
            if until > Instant::now() {
                return false;
            }
            entry.open_until = None;
        }
        true
    }

    pub fn penalty(&self, provider: &str) -> u32 {
        let entries = self.entries.lock().expect("health store poisoned");
        entries
            .get(provider)
            .map(|entry| entry.consecutive_failures)
            .unwrap_or(0)
    }

    pub fn record_success(&self, provider: &str, latency_ms: u128) {
        let mut entries = self.entries.lock().expect("health store poisoned");
        let entry = entries.entry(provider.to_string()).or_default();
        entry.successes += 1;
        entry.consecutive_failures = 0;
        entry.open_until = None;
        entry.last_error = None;
        entry.last_latency_ms = Some(latency_ms);
    }

    pub fn record_failure(&self, provider: &str, message: String) {
        let mut entries = self.entries.lock().expect("health store poisoned");
        let entry = entries.entry(provider.to_string()).or_default();
        entry.failures += 1;
        entry.consecutive_failures += 1;
        entry.last_error = Some(message);
        if entry.consecutive_failures >= self.threshold {
            entry.open_until = Some(Instant::now() + self.open_duration);
        }
    }

    pub fn snapshots(&self, provider_names: &[String]) -> Vec<HealthSnapshot> {
        let entries = self.entries.lock().expect("health store poisoned");
        provider_names
            .iter()
            .map(|provider| {
                let entry = entries.get(provider).cloned().unwrap_or_default();
                let remaining = entry
                    .open_until
                    .and_then(|until| until.checked_duration_since(Instant::now()));
                HealthSnapshot {
                    provider: provider.clone(),
                    successes: entry.successes,
                    failures: entry.failures,
                    consecutive_failures: entry.consecutive_failures,
                    circuit_open: remaining.is_some(),
                    circuit_remaining_ms: remaining.map(|duration| duration.as_millis()),
                    last_error: entry.last_error,
                    last_latency_ms: entry.last_latency_ms,
                }
            })
            .collect()
    }
}
