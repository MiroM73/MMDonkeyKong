//! The high-score screen: the top ten of one difficulty at a time, from the title screen.
//! Left / right (or a tap on a tab) switch the difficulty, Esc / Enter / Start / Back return.

use crate::app::screens::{ITEM_H, ITEM_W, MenuInput, menu_item};
use crate::app::*;
use crate::game::Difficulty;
use crate::save::HIGH_SCORES;

const BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 14);
const TEXT: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 135);
const GOLD: egui::Color32 = egui::Color32::from_rgb(240, 200, 60);
/// Width of the table and the height of one row (points).
const TABLE_W: f32 = 340.0;
const ROW_H: f32 = 28.0;

/// The next / previous difficulty in the tab row (stops at the ends).
pub(in crate::app) fn step_tab(d: Difficulty, dir: i32) -> Difficulty {
    let i = Difficulty::ALL.iter().position(|&x| x == d).unwrap_or(0) as i32;
    Difficulty::ALL[(i + dir).clamp(0, Difficulty::ALL.len() as i32 - 1) as usize]
}

impl App {
    pub(in crate::app) fn enter_high_scores(&mut self) {
        self.menu.scores = self.cfg.difficulty;
        self.screen = Screen::HighScores;
    }

    pub(in crate::app) fn high_scores_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let input = MenuInput::read(&ctx, &self.pad);
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, BG);
        self.menu.scores = step_tab(self.menu.scores, input.dx);
        let mut back = input.back || input.confirm;

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .id_salt("high_scores")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space(16.0);
        child.label(
            egui::RichText::new(t!("scores.title"))
                .size(30.0)
                .color(TEXT),
        );
        child.add_space(8.0);

        // The difficulty tabs.
        let tab_w = ((rect.width() - 24.0) / 5.0).clamp(60.0, 110.0);
        let width = 5.0 * tab_w + 4.0 * 6.0;
        let left = ((child.available_width() - width) / 2.0).max(0.0);
        child.horizontal(|ui| {
            ui.add_space(left);
            ui.spacing_mut().item_spacing.x = 6.0;
            for d in Difficulty::ALL {
                let size = egui::vec2(tab_w, 34.0);
                if menu_item(ui, &t!(d.key()), size, self.menu.scores == d, true).clicked() {
                    self.menu.scores = d;
                }
            }
        });
        child.add_space(12.0);

        let list = self
            .save
            .progress(self.menu.scores)
            .map(|p| p.high_scores.as_slice())
            .unwrap_or_default();
        for rank in 0..HIGH_SCORES {
            let (row, _) = child.allocate_exact_size(
                egui::vec2(TABLE_W.min(rect.width() - 16.0), ROW_H),
                egui::Sense::hover(),
            );
            let entry = list.get(rank);
            let ink = match (entry, rank) {
                (None, _) => TEXT_DIM,
                (Some(_), 0) => GOLD,
                _ => TEXT,
            };
            let font = egui::FontId::monospace(18.0);
            let p = child.painter();
            p.text(
                row.left_center() + egui::vec2(8.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{:>2}.", rank + 1),
                font.clone(),
                TEXT_DIM,
            );
            p.text(
                row.left_center() + egui::vec2(60.0, 0.0),
                egui::Align2::LEFT_CENTER,
                entry.map_or("---", |e| e.name.as_str()),
                font.clone(),
                ink,
            );
            p.text(
                row.right_center() - egui::vec2(8.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                entry.map_or("-".to_owned(), |e| e.score.to_string()),
                font,
                ink,
            );
        }
        child.add_space(10.0);
        let size = egui::vec2(ITEM_W.min(rect.width() - 16.0), ITEM_H);
        if menu_item(&mut child, &t!("levels.back"), size, true, true).clicked() {
            back = true;
        }
        if back {
            self.enter_title();
        }
        let wait = if self.pad.connected { 100 } else { 1000 };
        ctx.request_repaint_after(std::time::Duration::from_millis(wait));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tabs_stop_at_the_ends() {
        assert_eq!(step_tab(Difficulty::Relaxed, -1), Difficulty::Relaxed);
        assert_eq!(step_tab(Difficulty::Relaxed, 1), Difficulty::Easy);
        assert_eq!(step_tab(Difficulty::Hard, 1), Difficulty::Insane);
        assert_eq!(step_tab(Difficulty::Insane, 1), Difficulty::Insane);
        assert_eq!(step_tab(Difficulty::Normal, 0), Difficulty::Normal);
    }
}
