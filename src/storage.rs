//! Where saved data lives. The game code only knows the `Storage` trait; the desktop build keeps
//! one JSON file per key in `%APPDATA%\MMDonkeyKong\` (`FileStorage`), the web build `localStorage`
//! (`WebStorage`, keys `mmdonkeykong.<key>`).

#[cfg(test)]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

pub trait Storage {
    fn load(&self, key: &str) -> Option<String>;
    /// Stores `value`. On an error the previously stored value must stay untouched.
    fn save(&mut self, key: &str, value: &str) -> Result<(), String>;
}

/// Stores nothing (when the platform has no storage).
#[derive(Default)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
#[cfg(test)]
#[derive(Default)]
pub struct MemoryStorage(pub HashMap<String, String>);

#[cfg(test)]
impl Storage for MemoryStorage {
    fn load(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn save(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.0.insert(key.to_owned(), value.to_owned());
        Ok(())
    }
}

/// One file per key in a folder: key `save` -> `save.json`, key `save.bak` -> `save.json.bak`.
/// The folder is a parameter so the tests never touch `%APPDATA%`.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileStorage {
    dir: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileStorage {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, key: &str) -> PathBuf {
        match key.strip_suffix(".bak") {
            Some(base) => self.dir.join(format!("{base}.json.bak")),
            None => self.dir.join(format!("{key}.json")),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Storage for FileStorage {
    fn load(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.path(key)).ok()
    }

    /// Writes a temporary file and renames it over the real one, so a running second instance
    /// never reads a half-written file and a failed write leaves the old file untouched.
    fn save(&mut self, key: &str, value: &str) -> Result<(), String> {
        let p = self.path(key);
        write_atomic(&self.dir, &p, value).map_err(|e| e.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_atomic(dir: &Path, path: &Path, value: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(name);
    let result = std::fs::write(&tmp, value).and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// The browser's `localStorage`.
#[cfg(target_arch = "wasm32")]
pub struct WebStorage;

#[cfg(target_arch = "wasm32")]
impl WebStorage {
    fn area() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    fn name(key: &str) -> String {
        format!("mmdonkeykong.{key}")
    }
}

#[cfg(target_arch = "wasm32")]
impl Storage for WebStorage {
    fn load(&self, key: &str) -> Option<String> {
        Self::area()?.get_item(&Self::name(key)).ok().flatten()
    }

    fn save(&mut self, key: &str, value: &str) -> Result<(), String> {
        let area = Self::area().ok_or("localStorage is not available")?;
        area.set_item(&Self::name(key), value)
            .map_err(|e| format!("{e:?}"))
    }
}

/// The storage of this platform: files in the settings folder, or `localStorage`.
pub fn platform_storage() -> Box<dyn Storage> {
    #[cfg(target_arch = "wasm32")]
    return Box::new(WebStorage);
    #[cfg(not(target_arch = "wasm32"))]
    match crate::config::config_dir() {
        Some(dir) => Box::new(FileStorage::new(dir)),
        None => Box::new(NullStorage),
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

    /// A fresh empty folder under the system temp folder (never `%APPDATA%`).
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mmdk-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn file_storage_round_trip_and_names() {
        let dir = temp_dir("roundtrip");
        let mut s = FileStorage::new(&dir);
        assert_eq!(s.load("save"), None);
        s.save("save", "{}").unwrap();
        s.save("save.bak", "old").unwrap();
        assert_eq!(s.load("save").as_deref(), Some("{}"));
        assert!(dir.join("save.json").is_file());
        assert!(dir.join("save.json.bak").is_file());
        s.save("save", "[1]").unwrap();
        assert_eq!(s.load("save").as_deref(), Some("[1]"));
        // No temporary file is left behind.
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// When the temporary file cannot be written, the old save stays as it was.
    #[test]
    fn a_failed_write_leaves_the_old_file_untouched() {
        let dir = temp_dir("failed");
        let mut s = FileStorage::new(&dir);
        s.save("save", "good").unwrap();
        // A folder where the temporary file would go makes the write fail.
        let tmp = dir.join(format!("save.json.{}.tmp", std::process::id()));
        std::fs::create_dir(&tmp).unwrap();
        assert!(s.save("save", "new").is_err());
        assert_eq!(s.load("save").as_deref(), Some("good"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_that_cannot_be_created_is_an_error() {
        let dir = temp_dir("blocked");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file");
        std::fs::write(&file, "x").unwrap();
        let mut s = FileStorage::new(file.join("sub"));
        assert!(s.save("save", "x").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
