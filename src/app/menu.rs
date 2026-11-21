//! The menu bar: File, View (theme, language), Help.

use crate::app::*;

impl App {
    pub(in crate::app) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(t!("menu.file"), |ui| {
                if ui.button(t!("menu.file.exit")).clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
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
