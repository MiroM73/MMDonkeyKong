//! The About window (a tool window, #32).

use crate::app::native_window::ToolSpec;
use crate::app::*;

impl App {
    pub(in crate::app) fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_win.is_open() {
            return;
        }
        let native = self.cfg.separate_windows && !ctx.embed_viewports();
        let spec = ToolSpec {
            title: t!("about.title"),
            default_size: [420.0, 240.0],
            min_size: [300.0, 180.0],
        };
        let mut state = self.cfg.about_window.clone();
        let lang_code = &self.lang_code;
        self.about_win.show(ctx, native, &spec, &mut state, |ui| {
            ui.heading(base_title());
            ui.label(t!("about.text"));
            ui.label(t!("about.credits"));
            ui.label(t!("about.language", code = lang_code));
        });
        self.store_window_state(state, true);
    }
}
