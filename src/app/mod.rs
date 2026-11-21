//! The whole UI. `App` and the frame loop live here; every other file of this folder is an
//! `impl App` block for one area (menu, dialogs, ...).
//!
//! Conventions (see `docs/source-layout.md`):
//! - each submodule starts with `use crate::app::*;` (gets the imports, private types and consts
//!   of this file - a child module may use private items of its ancestors),
//! - methods used outside their own file are `pub(in crate::app) fn`,
//! - a new window goes into `dialogs/<name>.rs` (+ `mod <name>;` in `dialogs/mod.rs`),
//! - keep files under ~800 lines.

mod dialogs;
mod menu;

use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::Config;
use crate::{i18n, t};

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
    last_save: Instant,
    /// Code of the language in use, e.g. "sk".
    lang_code: String,
    show_about: bool,
    /// A message for the status bar (text, since when); cleared after a few seconds.
    status_info: Option<(String, Instant)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cfg, _first_run) = Config::load();
        let app = Self {
            lang_code: i18n::set_language(&cfg.language),
            cfg,
            cfg_dirty: false,
            last_save: Instant::now(),
            show_about: false,
            status_info: None,
        };
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
        if !self.cfg_dirty || (!force && self.last_save.elapsed() < SAVE_INTERVAL) {
            return;
        }
        match self.cfg.save() {
            Ok(()) => self.cfg_dirty = false,
            Err(e) => self.info(t!("error.save_settings", error = e)),
        }
        self.last_save = Instant::now();
    }

    fn info(&mut self, s: String) {
        self.status_info = Some((s, Instant::now()));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.track_zoom(&ctx);

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            if let Some((_, at)) = &self.status_info
                && at.elapsed() > Duration::from_secs(8)
            {
                self.status_info = None;
            }
            ui.label(self.status_info.as_ref().map_or("", |(s, _)| s.as_str()));
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(t!("app.name"));
            ui.label(t!("app.hello"));
        });

        self.about_window(&ctx);
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
