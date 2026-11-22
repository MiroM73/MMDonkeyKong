//! The whole UI. `App` and the frame loop live here; every other file of this folder is an
//! `impl App` block for one area (menu, dialogs, ...).
//!
//! Conventions (see `docs/source-layout.md`):
//! - each submodule starts with `use crate::app::*;` (gets the imports, private types and consts
//!   of this file - a child module may use private items of its ancestors),
//! - methods used outside their own file are `pub(in crate::app) fn`,
//! - a new window goes into `dialogs/<name>.rs` (+ `mod <name>;` in `dialogs/mod.rs`),
//! - keep files under ~800 lines.

mod controls;
mod dialogs;
mod game_view;
mod gamepad;
mod high_scores;
mod level_select;
mod menu;
mod name_entry;
mod native_window;
mod progress;
#[cfg(target_arch = "wasm32")]
mod pwa;
mod scene;
mod screens;
mod sprites;
mod touch_view;

use std::time::Duration;

use eframe::egui;

use crate::audio::{self, Audio};
use crate::config::Config;
use crate::save::SaveData;
use crate::storage::{self, Storage};
use crate::{i18n, t};
use game_view::GameSession;
use native_window::ToolWindow;
use screens::MenuState;

/// What the game area shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum Screen {
    Title,
    LevelSelect,
    HighScores,
    Game,
}

/// How often changed settings are written to disk at most.
const SAVE_INTERVAL: Duration = Duration::from_secs(2);
/// The range of the UI zoom (Ctrl + plus / minus / mouse wheel).
const UI_ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.5..=3.0;

/// Program name and version, e.g. "MMDonkeyKong v0.1.0".
pub fn base_title() -> String {
    format!("MMDonkeyKong v{}", env!("CARGO_PKG_VERSION"))
}

pub struct App {
    cfg: Config,
    /// Settings changed since the last write.
    cfg_dirty: bool,
    /// Time of the last settings write (egui time, seconds; `std::time::Instant` does not work
    /// in the browser).
    last_save: f64,
    /// Time of the current frame (egui time, seconds).
    now: f64,
    /// Code of the language in use, e.g. "sk".
    lang_code: String,
    about_win: ToolWindow,
    settings_win: ToolWindow,
    audio: Box<dyn Audio>,
    /// The volume and music state last given to `audio`.
    audio_applied: Option<(f32, bool)>,
    screen: Screen,
    game: GameSession,
    /// Saved progress and where it is kept.
    save: SaveData,
    storage: Box<dyn Storage>,
    menu: MenuState,
    /// The names of the levels (for the level select).
    level_names: Vec<String>,
    /// The gamepad state of this frame (polled once per frame).
    pad: gamepad::PadFrame,
    /// The live touches; "seen" turns the on-screen controls on in the automatic mode.
    touches: controls::Touches,
    gamepad: gamepad::Gamepad,
    sprites: sprites::Sprites,
    /// A message for the status bar (text, since when); cleared after a few seconds.
    status_info: Option<(String, f64)>,
    /// Browser: a new build of the web app is waiting (issue #18).
    #[cfg(target_arch = "wasm32")]
    update_watch: pwa::UpdateWatch,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cfg, _first_run) = Config::load();
        let difficulty = cfg.difficulty;
        let mut app = Self {
            lang_code: i18n::set_language(&cfg.language),
            cfg,
            cfg_dirty: false,
            last_save: cc.egui_ctx.input(|i| i.time),
            now: cc.egui_ctx.input(|i| i.time),
            about_win: ToolWindow::new("about_window"),
            settings_win: ToolWindow::new("settings_window"),
            audio: audio::platform_audio(),
            audio_applied: None,
            screen: Screen::Title,
            // Only the placeholder behind the title screen; every game is seeded from the clock.
            game: GameSession::new(1, 0, difficulty, None, 0),
            save: SaveData::default(),
            storage: storage::platform_storage(),
            menu: MenuState::default(),
            level_names: (0..crate::game::level_count())
                .map(|i| crate::game::builtin_level(i).name)
                .collect(),
            pad: gamepad::PadFrame::default(),
            touches: controls::Touches::default(),
            gamepad: gamepad::Gamepad::new(),
            sprites: sprites::Sprites::load(&cc.egui_ctx),
            status_info: None,
            #[cfg(target_arch = "wasm32")]
            update_watch: pwa::UpdateWatch::new(&cc.egui_ctx),
        };
        app.load_save();
        app.enter_title();
        app.apply_theme(&cc.egui_ctx);
        cc.egui_ctx.set_zoom_factor(app.cfg.ui_zoom);
        app
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let pref = match self.cfg.theme.as_str() {
            "dark" => egui::ThemePreference::Dark,
            "light" => egui::ThemePreference::Light,
            _ => egui::ThemePreference::System,
        };
        ctx.set_theme(pref);
    }

    /// Switches the UI language ("" = system language).
    fn set_language(&mut self, code: &str, reload_files: bool) {
        self.cfg.language = code.to_owned();
        self.lang_code = if reload_files {
            i18n::reload(code)
        } else {
            i18n::set_language(code)
        };
        self.cfg_dirty = true;
    }

    /// Remembers the UI zoom changed with Ctrl + plus / minus / wheel.
    fn track_zoom(&mut self, ctx: &egui::Context) {
        let zoom = ctx
            .zoom_factor()
            .clamp(*UI_ZOOM_RANGE.start(), *UI_ZOOM_RANGE.end());
        if (zoom - self.cfg.ui_zoom).abs() > 0.001 {
            self.cfg.ui_zoom = zoom;
            self.cfg_dirty = true;
        }
    }

    /// Writes the settings when they changed (at most every `SAVE_INTERVAL`, or `force`d).
    fn save_config(&mut self, force: bool) {
        if !self.cfg_dirty || (!force && self.now - self.last_save < SAVE_INTERVAL.as_secs_f64()) {
            return;
        }
        match self.cfg.save() {
            Ok(()) => self.cfg_dirty = false,
            Err(e) => self.info(t!("error.save_settings", error = e)),
        }
        self.last_save = self.now;
    }

    /// Gives the sound settings and the music state (music plays while a game runs) to the audio.
    fn sync_audio(&mut self) {
        let volume = if self.cfg.muted { 0.0 } else { self.cfg.volume };
        let music = volume > 0.0
            && self.cfg.music
            && self.screen == Screen::Game
            && !self.game.is_paused()
            && self.game.state.status == crate::game::Status::Playing;
        if self.audio_applied != Some((volume, music)) {
            self.audio.set_volume(volume);
            self.audio.set_music(music);
            self.audio_applied = Some((volume, music));
        }
    }

    /// Keeps the geometry / zoom of a tool window in the settings (`about` or Settings window).
    fn store_window_state(&mut self, state: crate::config::ToolWindowState, about: bool) {
        let slot = if about {
            &mut self.cfg.about_window
        } else {
            &mut self.cfg.settings_window
        };
        if *slot != state {
            *slot = state;
            self.cfg_dirty = true;
        }
    }

    fn info(&mut self, s: String) {
        self.status_info = Some((s, self.now));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.now = ctx.input(|i| i.time);
        self.track_zoom(&ctx);
        self.pad = self.gamepad.poll();

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            if let Some((_, at)) = &self.status_info
                && self.now - *at > 8.0
            {
                self.status_info = None;
            }
            match &self.status_info {
                Some((s, _)) => ui.label(s.as_str()),
                None => ui.label(t!("game.hint")),
            };
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| match self.screen {
                Screen::Title => self.title_ui(ui),
                Screen::LevelSelect => self.level_select_ui(ui),
                Screen::HighScores => self.high_scores_ui(ui),
                Screen::Game => self.game_ui(ui),
            });

        self.about_window(&ctx);
        self.settings_window(&ctx);
        self.sync_audio();
        self.save_config(false);
        // Keep the status message timeout and the delayed settings write running while idle.
        if self.cfg_dirty || self.status_info.is_some() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config(true);
    }
}
