//! Where saved data lives. The game code only knows this trait; the native build will use files
//! (`%APPDATA%\MMDonkeyKong\`) and the web build `localStorage` (issue #11).

use std::collections::HashMap;

pub trait Storage {
    fn load(&self, key: &str) -> Option<String>;
    fn save(&mut self, key: &str, value: &str) -> Result<(), String>;
}

/// Stores nothing (tests that must not touch the disk, or when no storage is available).
#[derive(Default)]
pub struct NullStorage;

impl Storage for NullStorage {
    fn load(&self, _key: &str) -> Option<String> {
        None
    }
    fn save(&mut self, _key: &str, _value: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Keeps the data in memory (tests of the save logic).
#[derive(Default)]
pub struct MemoryStorage(HashMap<String, String>);

impl Storage for MemoryStorage {
    fn load(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn save(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.0.insert(key.to_owned(), value.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_storage_round_trip() {
        let mut s = MemoryStorage::default();
        assert_eq!(s.load("a"), None);
        s.save("a", "1").unwrap();
        assert_eq!(s.load("a").as_deref(), Some("1"));
        assert_eq!(NullStorage.load("a"), None);
    }
}
