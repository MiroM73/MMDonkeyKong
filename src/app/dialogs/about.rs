//! The About window.

use crate::app::*;

impl App {
    pub(in crate::app) fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new(t!("about.title"))
            .id(egui::Id::unique("about_window"))
            .open(&mut self.show_about)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading(base_title());
                ui.label(t!("about.text"));
                ui.label(t!("about.credits"));
                ui.label(t!("about.language", code = &self.lang_code));
            });
    }
}
