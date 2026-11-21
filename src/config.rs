//! User settings, stored as JSON in `<config dir>/MMDonkeyKong/config.json`
//! (Windows: `%APPDATA%\MMDonkeyKong\config.json`).
//!
//! Every field has a default and the struct is `#[serde(default)]`, so a settings file written by
//! an older version (with fewer fields) still loads. Add new settings here with a default.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// "system", "light" or "dark".
    pub theme: String,
    /// Language code of the user interface; empty = the language of the operating system.
    pub language: String,
    /// UI zoom factor (Ctrl + plus / minus).
    pub ui_zoom: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: String::new(),
            ui_zoom: 1.0,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("MMDonkeyKong").join("config.json"))
    }

    /// Loads the settings. The second value is `false` on first run (no settings file yet).
    pub fn load() -> (Self, bool) {
        let text = Self::path().and_then(|p| std::fs::read_to_string(p).ok());
        let existed = text.is_some();
        let cfg = text.and_then(|s| Self::parse(&s)).unwrap_or_default();
        (cfg, existed)
    }

    fn parse(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    /// Writes the settings through a temporary file, so a second running instance never reads a
    /// half-written file.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(p) = Self::path() else { return Ok(()) };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = p.with_extension(format!("json.{}.tmp", std::process::id()));
        std::fs::write(&tmp, &json)?;
        if std::fs::rename(&tmp, &p).is_err() {
            let _ = std::fs::remove_file(&tmp);
            std::fs::write(&p, json)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An old / partial settings file loads, missing fields get their defaults.
    #[test]
    fn partial_file_gets_defaults() {
        let cfg = Config::parse(r#"{ "theme": "dark" }"#).unwrap();
        assert_eq!(cfg.theme, "dark");
        assert_eq!(cfg.ui_zoom, 1.0);
        assert_eq!(Config::parse("{}").unwrap(), Config::default());
    }

    #[test]
    fn broken_file_is_rejected() {
        assert!(Config::parse("not json").is_none());
    }
}
