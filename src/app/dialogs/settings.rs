//! The Settings window: sound, display, language, theme, difficulty and controls in one place.

use crate::app::*;
use crate::game::Difficulty;

impl App {
    pub(in crate::app) fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        egui::Window::new(t!("settings.title"))
            .id(egui::Id::unique("settings_window"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| self.settings_ui(ui));
        self.show_settings = open;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("settings.sound"));
        let mut changed = ui
            .checkbox(&mut self.cfg.muted, t!("settings.mute"))
            .changed();
        ui.add_enabled_ui(!self.cfg.muted, |ui| {
            ui.horizontal(|ui| {
                ui.label(t!("settings.volume"));
                let slider = egui::Slider::new(&mut self.cfg.volume, 0.0..=1.0).show_value(false);
                changed |= ui.add(slider).changed();
            });
            changed |= ui
                .checkbox(&mut self.cfg.music, t!("settings.music"))
                .changed();
        });

        ui.separator();
        ui.heading(t!("settings.display"));
        ui.horizontal(|ui| {
            ui.label(t!("settings.zoom"));
            let mut zoom = self.cfg.ui_zoom;
            if ui
                .add(egui::Slider::new(&mut zoom, UI_ZOOM_RANGE).fixed_decimals(1))
                .changed()
            {
                ui.ctx().set_zoom_factor(zoom);
            }
        });
        ui.horizontal(|ui| {
            ui.label(t!("settings.theme"));
            for (value, key) in [
                ("system", "theme.system"),
                ("light", "theme.light"),
                ("dark", "theme.dark"),
            ] {
                if ui.radio(self.cfg.theme == value, t!(key)).clicked() {
                    self.cfg.theme = value.to_owned();
                    self.apply_theme(ui.ctx());
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(t!("settings.language"));
            let current = if self.cfg.language.is_empty() {
                t!("lang.auto")
            } else {
                i18n::available()
                    .into_iter()
                    .find(|l| l.code == self.cfg.language)
                    .map_or_else(|| self.cfg.language.clone(), |l| l.name)
            };
            egui::ComboBox::from_id_salt("settings_language")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.cfg.language.is_empty(), t!("lang.auto"))
                        .clicked()
                    {
                        self.set_language("", false);
                    }
                    for l in i18n::available() {
                        if ui
                            .selectable_label(self.cfg.language == l.code, &l.name)
                            .clicked()
                        {
                            self.set_language(&l.code, false);
                        }
                    }
                });
        });

        ui.separator();
        ui.heading(t!("settings.difficulty"));
        ui.horizontal_wrapped(|ui| {
            for d in Difficulty::ALL {
                if ui.radio(self.cfg.difficulty == d, t!(d.key())).clicked() {
                    self.cfg.difficulty = d;
                    changed = true;
                }
            }
        });
        ui.label(t!("difficulty.note"));

        ui.separator();
        ui.heading(t!("settings.controls"));
        self.controls_widgets(ui);

        if changed {
            self.cfg_dirty = true;
        }
    }
}
