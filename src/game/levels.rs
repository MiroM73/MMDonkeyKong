//! The levels shipped with the game: the files of `levels/` embedded at build time.

use super::Level;

/// The level files in play order.
const FILES: &[&str] = &[include_str!("../../levels/01-barrels.txt")];

/// Number of built-in levels.
pub fn count() -> usize {
    FILES.len()
}

/// Level `index` (0-based). The files are checked by the tests (`every_level_file_is_valid`),
/// so a failure here is a bug in the build, not a user error.
pub fn builtin(index: usize) -> Level {
    let text = FILES[index];
    let level = Level::parse(text).expect("a built-in level parses (checked by the tests)");
    debug_assert_eq!(level.validate(), Ok(()));
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_file_is_valid() {
        for i in 0..count() {
            let l = builtin(i);
            assert_eq!(l.validate(), Ok(()), "{}", l.name);
        }
    }

    #[test]
    fn every_file_in_the_levels_folder_is_embedded_and_valid() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .collect();
        files.sort();
        assert_eq!(files.len(), count());
        for (i, path) in files.iter().enumerate() {
            let text = std::fs::read_to_string(path).unwrap();
            let level = Level::parse(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            assert_eq!(level.validate(), Ok(()), "{path:?}");
            assert_eq!(level, builtin(i), "{path:?} is not the embedded level {i}");
        }
    }
}
