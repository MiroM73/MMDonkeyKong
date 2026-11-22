//! Saved progress in the front end: loading the save at start-up, autosave after a completed
//! level and when a level is left through the menu, Continue / New game, what happens when a
//! level ends. The rules themselves are in `crate::save` (pure, tested).

use crate::app::game_view::GameSession;
use crate::app::name_entry::EntryNext;
use crate::app::*;
use crate::game::{self, Status};
use crate::save::{LoadError, SaveData};

/// A seed that differs on every launch: the wall clock (milliseconds) mixed with the frame time.
/// The frame time alone is the time since the start of the app, so it is nearly the same whenever
/// the player starts the first game quickly.
pub(in crate::app) fn clock_seed(frame_time: f64) -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    // `SystemTime` panics in the browser; `Date.now()` is the wall clock there.
    #[cfg(target_arch = "wasm32")]
    let wall = web_sys::js_sys::Date::now() as u64;
    wall ^ ((frame_time * 1000.0) as u64).rotate_left(32)
}

/// Message for a save that could not be read (the text was kept as a backup).
pub(in crate::app) fn load_problem(e: &LoadError) -> String {
    match e {
        LoadError::Damaged => t!("error.save_damaged"),
        LoadError::TooNew(v) => t!("error.save_too_new", version = v),
    }
}

impl App {
    /// Reads the save; a damaged one is set aside and reported in the status bar.
    pub(in crate::app) fn load_save(&mut self) {
        let (save, problem) = SaveData::load(&mut *self.storage, game::level_count());
        self.save = save;
        if let Some(e) = problem {
            self.info(load_problem(&e));
        }
    }

    /// Writes the save. A failing storage is reported and the game goes on; the old save stays.
    pub(in crate::app) fn persist(&mut self) {
        if let Err(e) = self.save.store(&mut *self.storage) {
            self.info(t!("error.save_game", error = e));
        }
    }

    /// Starts `level` (0-based) on the selected difficulty. `lives` is `None` for the full lives,
    /// `score` is what was carried over.
    pub(in crate::app) fn begin_level(
        &mut self,
        ctx: &egui::Context,
        level: usize,
        lives: Option<u32>,
        score: u32,
    ) {
        // The clock makes every game differ (a fixed seed would repeat the barrels).
        let seed = clock_seed(ctx.input(|i| i.time));
        self.game = GameSession::new(seed, level, self.cfg.difficulty, lives, score);
        // A finished game or a menu requests no repaints, so this frame's time is the whole idle
        // wait (it would look like a stall and pause the new game): drop it.
        self.game.skip_dt = true;
        self.screen = Screen::Game;
    }

    /// Title -> Continue: the saved level start of the selected difficulty.
    pub(in crate::app) fn continue_game(&mut self, ctx: &egui::Context) {
        if let Some(r) = self.save.resume(self.cfg.difficulty) {
            self.begin_level(ctx, r.level, Some(r.lives), r.score);
        }
    }

    /// Title -> New game: level 1, the full lives, no score; the continue point starts over.
    pub(in crate::app) fn new_game(&mut self, ctx: &egui::Context) {
        let d = self.cfg.difficulty;
        self.save.new_game(d, d.params().lives);
        self.persist();
        self.begin_level(ctx, 0, None, 0);
    }

    /// The level just ended with `Status::Complete`: unlock it, keep the score, move Continue
    /// on, and autosave.
    pub(in crate::app) fn on_level_complete(&mut self) {
        let st = &self.game.state;
        let (d, score, lives) = (st.difficulty, st.score, st.lives);
        self.save
            .complete_level(d, self.game.level, score, lives, game::level_count());
        self.persist();
    }

    /// Is the level that just ended the last one (finishing it finishes the game)?
    pub(in crate::app) fn is_final_level(&self) -> bool {
        self.game.level + 1 >= game::level_count()
    }

    /// Enter / tap after the level ended: game over restarts the level with the score at zero,
    /// a completed level goes on to the next one, the last one asks what to do next.
    pub(in crate::app) fn after_level(&mut self, ctx: &egui::Context) {
        let st = &self.game.state;
        let level = self.game.level;
        match st.status {
            Status::Playing => {}
            Status::GameOver => self.finish_run(ctx, st.score, EntryNext::Restart),
            Status::Complete if !self.is_final_level() => {
                let (lives, score) = (st.lives, st.score);
                self.begin_level(ctx, level + 1, Some(lives), score);
            }
            Status::Complete => self.finish_run(ctx, st.score, EntryNext::Victory),
        }
        // The menu that may have opened is drawn in the next frame.
        ctx.request_repaint();
    }
}
