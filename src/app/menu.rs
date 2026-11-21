//! The menu bar: File, View (theme, language), Help.

use crate::app::*;

impl App {
    pub(in crate::app) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            // A web page cannot be closed by the program, so the File menu (only Exit) is desktop-only.
            #[cfg(not(target_arch = "wasm32"))]
            ui.menu_button(t!("menu.file"), |ui| {
                if ui.button(t!("menu.file.exit")).clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(t!("menu.game"), |ui| self.difficulty_menu(ui));
            ui.menu_button(t!("menu.view"), |ui| {
                ui.menu_button(t!("menu.view.theme"), |ui| {
                    for (value, key) in [
                        ("system", "theme.system"),
                        ("light", "theme.light"),
                        ("dark", "theme.dark"),
                    ] {
                        if ui.radio(self.cfg.theme == value, t!(key)).clicked() {
                            ui.close();
                            self.cfg.theme = value.to_owned();
                            self.apply_theme(ui.ctx());
                            self.cfg_dirty = true;
                        }
                    }
                });
                self.controls_menu(ui);
                self.language_menu(ui);
            });
            ui.menu_button(t!("menu.help"), |ui| {
                if ui.button(t!("menu.help.about")).clicked() {
                    ui.close();
                    self.show_about = true;
                }
            });
        });
    }

    /// Game → Difficulty: applies from the next level start (a running level keeps its numbers).
    fn difficulty_menu(&mut self, ui: &mut egui::Ui) {
        for d in Difficulty::ALL {
            if ui.radio(self.cfg.difficulty == d, t!(d.key())).clicked() {
                self.cfg.difficulty = d;
                self.cfg_dirty = true;
            }
        }
        ui.separator();
        ui.label(t!("difficulty.note"));
    }

    /// View → Controls: when the on-screen buttons show, their size and side, vibration.
    fn controls_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(t!("menu.view.controls"), |ui| {
            for (value, key) in [
                ("auto", "controls.auto"),
                ("on", "controls.on"),
                ("off", "controls.off"),
            ] {
                if ui
                    .radio(self.cfg.touch_controls == value, t!(key))
                    .clicked()
                {
                    self.cfg.touch_controls = value.to_owned();
                    self.cfg_dirty = true;
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(t!("controls.size"));
                let slider =
                    egui::Slider::new(&mut self.cfg.touch_size, 0.6..=1.6).show_value(false);
                if ui.add(slider).changed() {
                    self.cfg_dirty = true;
                }
            });
            if ui
                .checkbox(&mut self.cfg.left_handed, t!("controls.left_handed"))
                .changed()
            {
                self.cfg_dirty = true;
            }
            if ui
                .checkbox(&mut self.cfg.vibration, t!("controls.vibration"))
                .changed()
            {
                self.cfg_dirty = true;
            }
        });
    }

    /// View → Language: automatic, every available language, reload of the language files.
    fn language_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(t!("menu.view.language"), |ui| {
            if ui
                .radio(self.cfg.language.is_empty(), t!("lang.auto"))
                .clicked()
            {
                ui.close();
                self.set_language("", false);
            }
            for l in i18n::available() {
                if ui.radio(self.cfg.language == l.code, &l.name).clicked() {
                    ui.close();
                    self.set_language(&l.code, false);
                }
            }
            ui.separator();
            if ui.button(t!("lang.reload")).clicked() {
                ui.close();
                let code = self.cfg.language.clone();
                self.set_language(&code, true);
            }
        });
    }
}
