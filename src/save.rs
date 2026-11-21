//! The saved progress: pure data and rules, no I/O (the `Storage` trait does the reading and
//! writing). Shared by all difficulties: the set of completed levels (completing a level on any
//! difficulty unlocks it for all). Per difficulty: the continue point, the best score of every
//! level, the high scores. The file carries a `version` and is migrated; a damaged file is
//! ignored and kept as `save.bak`. See `docs/architecture.md` -> Saved progress.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::game::Difficulty;
use crate::storage::Storage;

/// The storage key of the save (`save.json`, `localStorage["mmdonkeykong.save"]`).
pub const SAVE_KEY: &str = "save";
/// Where a damaged (or too new) save is kept before it is overwritten.
pub const BACKUP_KEY: &str = "save.bak";
/// The current format. Version 0 is a file without a `version` field.
pub const SAVE_VERSION: u32 = 1;
/// Lives above this are treated as a damaged value.
const MAX_LIVES: u32 = 99;
/// Entries of a high-score table.
pub const HIGH_SCORES: usize = 10;

/// Where "Continue" resumes: the start of a level with the lives and the score carried into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resume {
    pub level: usize,
    pub lives: u32,
    pub score: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighScore {
    pub name: String,
    pub score: u32,
}

/// What is kept per difficulty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub resume: Option<Resume>,
    /// Best score of every level completed on this difficulty.
    pub best: BTreeMap<usize, u32>,
    /// Best first, at most `HIGH_SCORES`.
    pub high_scores: Vec<HighScore>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    /// Level indices completed on any difficulty.
    pub completed: BTreeSet<usize>,
    pub difficulties: BTreeMap<Difficulty, Progress>,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            completed: BTreeSet::new(),
            difficulties: BTreeMap::new(),
        }
    }
}

/// Is level `level` (0-based) of `count` levels playable? The first one always, the completed
/// ones and the next one after a completed one.
pub fn is_playable(completed: &BTreeSet<usize>, level: usize, count: usize) -> bool {
    level < count && (level == 0 || completed.contains(&level) || completed.contains(&(level - 1)))
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// Not valid JSON / not the expected shape.
    Damaged,
    /// Written by a newer version of the game.
    TooNew(u32),
}

impl SaveData {
    pub fn progress(&self, d: Difficulty) -> Option<&Progress> {
        self.difficulties.get(&d)
    }

    pub fn progress_mut(&mut self, d: Difficulty) -> &mut Progress {
        self.difficulties.entry(d).or_default()
    }

    pub fn resume(&self, d: Difficulty) -> Option<Resume> {
        self.progress(d).and_then(|p| p.resume)
    }

    pub fn best(&self, d: Difficulty, level: usize) -> Option<u32> {
        self.progress(d).and_then(|p| p.best.get(&level).copied())
    }

    pub fn is_playable(&self, level: usize, count: usize) -> bool {
        is_playable(&self.completed, level, count)
    }

    /// A new game on `d`: the continue point is the first level with the full lives.
    pub fn new_game(&mut self, d: Difficulty, lives: u32) {
        self.progress_mut(d).resume = Some(Resume {
            level: 0,
            lives,
            score: 0,
        });
    }

    /// Level `level` was completed on `d` with `score` and `lives` left. The level is unlocked
    /// for every difficulty, the best score is kept, and Continue moves to the next level -
    /// only forward, so replaying an earlier level never lowers it. After the last level the
    /// game is finished: there is nothing to continue.
    pub fn complete_level(
        &mut self,
        d: Difficulty,
        level: usize,
        score: u32,
        lives: u32,
        count: usize,
    ) {
        self.completed.insert(level);
        let p = self.progress_mut(d);
        let best = p.best.entry(level).or_insert(0);
        *best = (*best).max(score);
        if level + 1 >= count {
            p.resume = None;
        } else if p.resume.is_none_or(|r| level + 1 > r.level) {
            p.resume = Some(Resume {
                level: level + 1,
                lives,
                score,
            });
        }
    }

    /// All lives were lost on `level`: the level restarts with the full lives and no score, and
    /// when that was the continue point, Continue restarts there too.
    pub fn game_over(&mut self, d: Difficulty, level: usize, lives: u32) {
        if let Some(r) = &mut self.progress_mut(d).resume
            && r.level == level
        {
            *r = Resume {
                level,
                lives,
                score: 0,
            };
        }
    }

    /// Reads a save. Unversioned text is version 0 (the same shape without `version`) and is
    /// migrated; a newer version or garbage is an error and the caller keeps the text as backup.
    pub fn parse(text: &str) -> Result<Self, LoadError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|_| LoadError::Damaged)?;
        let version = match value.get("version") {
            None => 0,
            Some(v) => v.as_u64().ok_or(LoadError::Damaged)? as u32,
        };
        if version > SAVE_VERSION {
            return Err(LoadError::TooNew(version));
        }
        let mut save: SaveData = serde_json::from_value(value).map_err(|_| LoadError::Damaged)?;
        // Migrations: 0 -> 1 only stamps the version (same shape).
        save.version = SAVE_VERSION;
        Ok(save)
    }

    /// Repairs values that cannot be valid for a game with `count` levels (a hand-edited or
    /// damaged file must not crash the game).
    pub fn sanitize(&mut self, count: usize) {
        for p in self.difficulties.values_mut() {
            if let Some(r) = &mut p.resume {
                if r.level >= count || r.lives == 0 {
                    p.resume = None;
                } else {
                    r.lives = r.lives.min(MAX_LIVES);
                }
            }
            p.high_scores.sort_by_key(|h| std::cmp::Reverse(h.score));
            p.high_scores.truncate(HIGH_SCORES);
        }
    }

    /// Loads the save from `storage`. The second value says what went wrong, if anything: the
    /// text of a damaged / too new save is then kept under `save.bak` and the game starts fresh.
    pub fn load(storage: &mut dyn Storage, count: usize) -> (Self, Option<LoadError>) {
        let Some(text) = storage.load(SAVE_KEY) else {
            return (Self::default(), None);
        };
        match Self::parse(&text) {
            Ok(mut save) => {
                save.sanitize(count);
                (save, None)
            }
            Err(e) => {
                let _ = storage.save(BACKUP_KEY, &text);
                (Self::default(), Some(e))
            }
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        storage.save(SAVE_KEY, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    const D: Difficulty = Difficulty::Normal;

    #[test]
    fn round_trip() {
        let mut s = SaveData::default();
        s.new_game(D, 3);
        s.complete_level(D, 0, 1200, 2, 8);
        s.complete_level(Difficulty::Hard, 1, 500, 3, 8);
        s.progress_mut(D).high_scores.push(HighScore {
            name: "MIR".into(),
            score: 900,
        });
        let mut mem = MemoryStorage::default();
        s.store(&mut mem).unwrap();
        let (loaded, err) = SaveData::load(&mut mem, 8);
        assert_eq!(err, None);
        assert_eq!(loaded, s);
    }

    #[test]
    fn nothing_saved_gives_an_empty_save() {
        let mut mem = MemoryStorage::default();
        let (s, err) = SaveData::load(&mut mem, 8);
        assert_eq!((s, err), (SaveData::default(), None));
    }

    /// A file without a version (version 0, even an empty or partial one) loads and is stamped.
    #[test]
    fn an_old_version_migrates() {
        let s = SaveData::parse(r#"{ "completed": [0, 1] }"#).unwrap();
        assert_eq!(s.version, SAVE_VERSION);
        assert_eq!(s.completed, BTreeSet::from([0, 1]));
        assert_eq!(SaveData::parse("{}").unwrap(), SaveData::default());
        let v0 = r#"{"version":0,"difficulties":{"hard":{"best":{"0":700}}}}"#;
        let s = SaveData::parse(v0).unwrap();
        assert_eq!(s.best(Difficulty::Hard, 0), Some(700));
    }

    #[test]
    fn a_newer_version_is_refused_and_kept() {
        let text = r#"{"version":99,"completed":[0]}"#;
        assert_eq!(SaveData::parse(text), Err(LoadError::TooNew(99)));
        let mut mem = MemoryStorage::default();
        mem.save(SAVE_KEY, text).unwrap();
        let (s, err) = SaveData::load(&mut mem, 8);
        assert_eq!(err, Some(LoadError::TooNew(99)));
        assert_eq!(s, SaveData::default());
        assert_eq!(mem.load(BACKUP_KEY).as_deref(), Some(text));
    }

    #[test]
    fn corrupt_data_does_not_crash_and_is_kept_as_backup() {
        for bad in [
            "",
            "not json",
            "[1, 2]",
            r#"{"version": "x"}"#,
            r#"{"completed": ["a"]}"#,
            r#"{"difficulties": {"nightmare": {}}}"#,
            "\u{0}\u{1}",
        ] {
            let mut mem = MemoryStorage::default();
            mem.save(SAVE_KEY, bad).unwrap();
            let (s, err) = SaveData::load(&mut mem, 8);
            assert_eq!(err, Some(LoadError::Damaged), "{bad:?}");
            assert_eq!(s, SaveData::default());
            assert_eq!(mem.load(BACKUP_KEY).as_deref(), Some(bad));
        }
    }

    #[test]
    fn impossible_values_are_repaired() {
        let text = r#"{"difficulties":{
            "easy":{"resume":{"level":50,"lives":3,"score":1}},
            "hard":{"resume":{"level":1,"lives":0,"score":1}},
            "normal":{"resume":{"level":1,"lives":500,"score":1}}}}"#;
        let mut mem = MemoryStorage::default();
        mem.save(SAVE_KEY, text).unwrap();
        let (s, err) = SaveData::load(&mut mem, 8);
        assert_eq!(err, None);
        assert_eq!(s.resume(Difficulty::Easy), None);
        assert_eq!(s.resume(Difficulty::Hard), None);
        assert_eq!(s.resume(Difficulty::Normal).unwrap().lives, MAX_LIVES);
    }

    /// A storage that cannot write: the game goes on and reports the error; what was stored
    /// before stays.
    struct ReadOnly(MemoryStorage);
    impl Storage for ReadOnly {
        fn load(&self, key: &str) -> Option<String> {
            self.0.load(key)
        }
        fn save(&mut self, _: &str, _: &str) -> Result<(), String> {
            Err("disk full".into())
        }
    }

    #[test]
    fn a_failing_write_is_an_error_and_keeps_the_old_save() {
        let mut good = SaveData::default();
        good.complete_level(D, 0, 10, 3, 8);
        let mut mem = MemoryStorage::default();
        good.store(&mut mem).unwrap();
        let mut ro = ReadOnly(mem);
        let mut newer = good.clone();
        newer.complete_level(D, 1, 99, 3, 8);
        assert_eq!(newer.store(&mut ro), Err("disk full".to_owned()));
        assert_eq!(SaveData::load(&mut ro, 8).0, good);
    }

    #[test]
    fn unlocking_is_shared_and_the_next_level_is_playable() {
        let mut s = SaveData::default();
        assert!(s.is_playable(0, 8));
        assert!(!s.is_playable(1, 8));
        s.complete_level(Difficulty::Insane, 0, 10, 2, 8);
        // Completed on Insane: unlocked for every difficulty.
        assert!(s.is_playable(0, 8) && s.is_playable(1, 8));
        assert!(!s.is_playable(2, 8));
        s.complete_level(Difficulty::Relaxed, 1, 10, 2, 8);
        assert!(s.is_playable(2, 8) && !s.is_playable(3, 8));
        // Never beyond the number of levels.
        assert!(!is_playable(&BTreeSet::from([7]), 8, 8));
        assert!(!is_playable(&BTreeSet::new(), 0, 0));
    }

    #[test]
    fn completing_keeps_the_best_score_and_moves_continue_forward_only() {
        let mut s = SaveData::default();
        s.new_game(D, 3);
        s.complete_level(D, 0, 900, 2, 3);
        let expected = Resume {
            level: 1,
            lives: 2,
            score: 900,
        };
        assert_eq!(s.resume(D), Some(expected));
        s.complete_level(D, 1, 2000, 1, 3);
        assert_eq!(s.resume(D).unwrap().level, 2);
        // Replaying level 0 never lowers Continue or the best score.
        s.complete_level(D, 0, 300, 3, 3);
        assert_eq!(s.resume(D).unwrap().level, 2);
        assert_eq!(s.best(D, 0), Some(900));
        s.complete_level(D, 0, 1500, 3, 3);
        assert_eq!(s.best(D, 0), Some(1500));
        // The last level finishes the game.
        s.complete_level(D, 2, 5000, 1, 3);
        assert_eq!(s.resume(D), None);
        assert_eq!(s.completed, BTreeSet::from([0, 1, 2]));
        // Other difficulties are untouched.
        assert_eq!(s.best(Difficulty::Easy, 0), None);
    }

    #[test]
    fn game_over_restarts_the_level_with_no_score() {
        let mut s = SaveData::default();
        s.complete_level(D, 0, 900, 1, 3);
        s.game_over(D, 1, 3);
        let expected = Resume {
            level: 1,
            lives: 3,
            score: 0,
        };
        assert_eq!(s.resume(D), Some(expected));
        // A game over in a replayed earlier level leaves Continue alone.
        s.game_over(D, 0, 3);
        assert_eq!(s.resume(D).unwrap().level, 1);
    }
}
