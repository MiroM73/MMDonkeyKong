//! The level-select screen: a grid of level tiles (number, name, best score on the selected
//! difficulty, one medal per difficulty). Completed levels and the next one are playable, later
//! ones are greyed out with a lock. Keyboard / gamepad move a cursor, a tap or click starts a level.

use crate::app::screens::{ITEM_H, ITEM_W, MenuInput, hover_moves, menu_item, move_cursor};
use crate::app::*;
use crate::game::{self, Difficulty};
use crate::save::SaveData;

const BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 14);
const TEXT: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 135);
const TILE: egui::Color32 = egui::Color32::from_rgb(34, 36, 70);
const TILE_DONE: egui::Color32 = egui::Color32::from_rgb(30, 62, 56);
const TILE_LOCKED: egui::Color32 = egui::Color32::from_rgb(24, 24, 34);
const SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 60, 60);
/// The colour of the medal of every difficulty, easiest first.
const MEDALS: [egui::Color32; 5] = [
    egui::Color32::from_rgb(120, 200, 120),
    egui::Color32::from_rgb(90, 170, 230),
    egui::Color32::from_rgb(240, 200, 60),
    egui::Color32::from_rgb(240, 140, 50),
    egui::Color32::from_rgb(230, 70, 70),
];
/// Tile size and the gap between tiles (points): big enough for a thumb.
const TILE_W: f32 = 132.0;
const TILE_H: f32 = 116.0;
const GAP: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum TileState {
    /// Not reachable yet.
    Locked,
    /// Playable, not completed on any difficulty.
    Open,
    /// Completed on some difficulty.
    Done,
}

/// What one tile shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) struct TileInfo {
    pub state: TileState,
    /// Best score on the selected difficulty.
    pub best: Option<u32>,
    /// Completed on this difficulty (easiest first).
    pub medals: [bool; 5],
}

/// The tile of `level` for the selected difficulty `d`.
pub(in crate::app) fn tile_info(
    save: &SaveData,
    level: usize,
    count: usize,
    d: Difficulty,
) -> TileInfo {
    let state = if !save.is_playable(level, count) {
        TileState::Locked
    } else if save.completed.contains(&level) {
        TileState::Done
    } else {
        TileState::Open
    };
    let mut medals = [false; 5];
    for (i, other) in Difficulty::ALL.iter().enumerate() {
        medals[i] = save.best(*other, level).is_some();
    }
    TileInfo {
        state,
        best: save.best(d, level),
        medals,
    }
}

/// How many tiles fit in a row of `avail` points (at least one, never more than `count`).
pub(in crate::app) fn grid_cols(avail: f32, tile: f32, gap: f32, count: usize) -> usize {
    let fit = ((avail + gap) / (tile + gap)).floor();
    let fit = if fit.is_finite() && fit >= 1.0 {
        fit as usize
    } else {
        1
    };
    fit.min(count.max(1))
}

impl App {
    /// Opens the level select with the cursor on the level to play next.
    pub(in crate::app) fn enter_level_select(&mut self) {
        let count = game::level_count();
        let next = self
            .save
            .resume(self.cfg.difficulty)
            .map(|r| r.level)
            .unwrap_or_else(|| {
                (0..count)
                    .rev()
                    .find(|&l| self.save.is_playable(l, count))
                    .unwrap_or(0)
            });
        self.menu.level = next.min(count.saturating_sub(1));
        self.screen = Screen::LevelSelect;
    }

    pub(in crate::app) fn level_select_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let input = MenuInput::read(&ctx, &self.pad);
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, BG);
        let count = game::level_count();
        let d = self.cfg.difficulty;

        let cols = grid_cols(rect.width() - 2.0 * GAP, TILE_W, GAP, count);
        let before = self.menu.level;
        self.menu.level = move_cursor(self.menu.level, count, cols, input.dx, input.dy);
        // The cursor stays on tiles that can be played.
        if !self.save.is_playable(self.menu.level, count) {
            self.menu.level = before;
        }
        let moved = self.menu.level != before;

        let mut start = None;
        let mut back = input.back;
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .id_salt("level_select")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space(16.0);
        child.label(
            egui::RichText::new(t!("levels.title"))
                .size(30.0)
                .color(TEXT),
        );
        child.label(
            egui::RichText::new(t!("title.difficulty", name = t!(d.key())))
                .size(15.0)
                .color(TEXT_DIM),
        );
        child.add_space(8.0);
        let back_h = ITEM_H + 24.0;
        let grid_h = (child.available_height() - back_h).max(TILE_H);
        egui::ScrollArea::vertical()
            .id_salt("level_grid")
            .max_height(grid_h)
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
                let grid_w = cols as f32 * (TILE_W + GAP) - GAP;
                let pad = ((ui.available_width() - grid_w) / 2.0).max(0.0);
                for row in (0..count).step_by(cols) {
                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        for level in row..(row + cols).min(count) {
                            let info = tile_info(&self.save, level, count, d);
                            let selected = self.menu.level == level;
                            let resp = self.draw_tile(ui, level, &info, selected);
                            if moved && selected {
                                resp.scroll_to_me(None);
                            }
                            if info.state != TileState::Locked && hover_moves(ui, &resp) {
                                self.menu.level = level;
                            }
                            if info.state != TileState::Locked && resp.clicked() {
                                start = Some(level);
                            }
                        }
                    });
                }
            });
        child.add_space(6.0);
        draw_legend(&mut child);
        child.add_space(8.0);
        let size = egui::vec2(ITEM_W.min(rect.width() - 16.0), ITEM_H);
        if menu_item(&mut child, &t!("levels.back"), size, false, true).clicked() {
            back = true;
        }

        let sel = self.menu.level;
        if input.confirm && tile_info(&self.save, sel, count, d).state != TileState::Locked {
            start = Some(sel);
        }
        if let Some(level) = start {
            self.begin_level(&ctx, level, None, 0);
        } else if back {
            self.enter_title();
        }
        let wait = if self.pad.connected { 100 } else { 1000 };
        ctx.request_repaint_after(std::time::Duration::from_millis(wait));
    }

    fn draw_tile(
        &self,
        ui: &mut egui::Ui,
        level: usize,
        info: &TileInfo,
        selected: bool,
    ) -> egui::Response {
        let locked = info.state == TileState::Locked;
        let sense = if locked {
            egui::Sense::hover()
        } else {
            egui::Sense::click()
        };
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(TILE_W, TILE_H), sense);
        let fill = match info.state {
            TileState::Locked => TILE_LOCKED,
            TileState::Open => TILE,
            TileState::Done => TILE_DONE,
        };
        let p = ui.painter();
        p.rect_filled(rect, 8.0, fill);
        if selected {
            p.rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(3.0, SELECTED),
                egui::StrokeKind::Inside,
            );
        }
        let ink = if locked { TEXT_DIM } else { TEXT };
        p.text(
            rect.left_top() + egui::vec2(10.0, 6.0),
            egui::Align2::LEFT_TOP,
            (level + 1).to_string(),
            egui::FontId::proportional(26.0),
            ink,
        );
        if locked {
            draw_lock(p, rect.right_top() + egui::vec2(-24.0, 24.0), TEXT_DIM);
        } else {
            let name = self.level_names.get(level).map_or("", String::as_str);
            p.text(
                rect.center() + egui::vec2(0.0, -2.0),
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(15.0),
                ink,
            );
            let best = match info.best {
                Some(b) => t!("levels.best", score = b),
                None => t!("levels.no_score"),
            };
            p.text(
                rect.center() + egui::vec2(0.0, 18.0),
                egui::Align2::CENTER_CENTER,
                best,
                egui::FontId::proportional(13.0),
                TEXT_DIM,
            );
            for (i, done) in info.medals.iter().enumerate() {
                let c = egui::pos2(
                    rect.center().x + (i as f32 - 2.0) * 18.0,
                    rect.bottom() - 14.0,
                );
                if *done {
                    p.circle_filled(c, 6.0, MEDALS[i]);
                } else {
                    p.circle_stroke(c, 5.5, egui::Stroke::new(1.5, TEXT_DIM));
                }
                let hit = egui::Rect::from_center_size(c, egui::vec2(18.0, 20.0));
                let tip = ui.interact(hit, resp.id.with(("medal", i)), egui::Sense::hover());
                let name = t!(Difficulty::ALL[i].key());
                tip.on_hover_text(if *done {
                    t!("levels.medal_done", name = name)
                } else {
                    t!("levels.medal_todo", name = name)
                });
            }
        }
        resp
    }
}

/// The key under the grid: what the coloured circles of a tile mean.
fn draw_legend(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(t!("levels.legend"))
            .size(13.0)
            .color(TEXT_DIM),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        // Centre the row: the wrapped layout starts at the left edge.
        let names: Vec<String> = Difficulty::ALL.iter().map(|d| t!(d.key())).collect();
        let text_w: f32 = names.iter().map(|n| n.chars().count() as f32 * 7.5).sum();
        let total = text_w + names.len() as f32 * 30.0;
        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
        for (i, name) in names.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 14.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 6.0, MEDALS[i]);
            ui.label(egui::RichText::new(name).size(13.0).color(TEXT_DIM));
            ui.add_space(8.0);
        }
    });
}

/// A padlock centred at `c`.
fn draw_lock(p: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    let body = egui::Rect::from_center_size(c + egui::vec2(0.0, 4.0), egui::vec2(18.0, 13.0));
    p.rect_filled(body, 2.0, color);
    let arc: Vec<egui::Pos2> = (0..=10)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32 / 10.0);
            egui::pos2(c.x - 5.5 * a.cos(), body.top() - 5.5 * a.sin())
        })
        .collect();
    p.add(egui::Shape::line(arc, egui::Stroke::new(2.5, color)));
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Difficulty = Difficulty::Normal;

    #[test]
    fn locked_open_and_done_tiles() {
        let mut s = SaveData::default();
        let st = |s: &SaveData, l| tile_info(s, l, 8, D).state;
        assert_eq!(st(&s, 0), TileState::Open);
        assert_eq!(st(&s, 1), TileState::Locked);
        s.complete_level(Difficulty::Hard, 0, 500, 2, 8);
        // Completed on Hard: done for every difficulty, and the next level is open.
        assert_eq!(st(&s, 0), TileState::Done);
        assert_eq!(st(&s, 1), TileState::Open);
        assert_eq!(st(&s, 2), TileState::Locked);
        // Replaying never locks anything again.
        s.complete_level(D, 0, 100, 1, 8);
        assert_eq!(st(&s, 1), TileState::Open);
    }

    #[test]
    fn medals_and_best_score_are_per_difficulty() {
        let mut s = SaveData::default();
        s.complete_level(Difficulty::Hard, 0, 500, 2, 8);
        s.complete_level(Difficulty::Easy, 0, 900, 2, 8);
        let t = tile_info(&s, 0, 8, Difficulty::Hard);
        assert_eq!(t.medals, [false, true, false, true, false]);
        assert_eq!(t.best, Some(500));
        assert_eq!(tile_info(&s, 0, 8, Difficulty::Easy).best, Some(900));
        assert_eq!(tile_info(&s, 0, 8, Difficulty::Normal).best, None);
    }

    #[test]
    fn the_grid_has_at_least_one_and_at_most_count_columns() {
        assert_eq!(grid_cols(10.0, 132.0, 12.0, 8), 1);
        assert_eq!(grid_cols(0.0, 132.0, 12.0, 8), 1);
        assert_eq!(grid_cols(f32::NAN, 132.0, 12.0, 8), 1);
        assert_eq!(grid_cols(600.0, 132.0, 12.0, 8), 4); // 4 * 132 + 3 * 12 = 564
        assert_eq!(grid_cols(5000.0, 132.0, 12.0, 8), 8);
        assert_eq!(grid_cols(5000.0, 132.0, 12.0, 0), 1);
    }
}
