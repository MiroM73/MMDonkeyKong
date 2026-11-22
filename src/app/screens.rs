//! The menu screens drawn in the game area: the title screen, the pause menu and the question
//! after the last level. They share one input (`MenuInput`: keyboard, gamepad and pointer / touch)
//! and one cursor model, so every item can be reached without a mouse.

use crate::app::gamepad::PadFrame;
use crate::app::*;
use crate::game::Difficulty;

const BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 14);
const ITEM: egui::Color32 = egui::Color32::from_rgb(34, 36, 70);
const ITEM_SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 60, 60);
const ITEM_OFF: egui::Color32 = egui::Color32::from_rgb(24, 24, 36);
const TEXT: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
const TEXT_OFF: egui::Color32 = egui::Color32::from_rgb(110, 110, 125);
/// Width and height of a menu item (points); big enough for a thumb.
pub(in crate::app) const ITEM_W: f32 = 300.0;
pub(in crate::app) const ITEM_H: f32 = 44.0;

/// What the user asked for in this frame, whatever the device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) struct MenuInput {
    /// -1 / 0 / 1: a direction pressed in this frame.
    pub dx: i32,
    pub dy: i32,
    /// Enter, Space, the gamepad's A.
    pub confirm: bool,
    /// Esc, the gamepad's Start.
    pub back: bool,
}

impl MenuInput {
    pub(in crate::app) fn read(ctx: &egui::Context, pad: &PadFrame) -> Self {
        ctx.input(|i| {
            let key = |keys: &[egui::Key]| keys.iter().any(|&k| i.key_pressed(k));
            let axis = |neg: bool, pos: bool| i32::from(pos) - i32::from(neg);
            MenuInput {
                dx: axis(
                    key(&[egui::Key::ArrowLeft, egui::Key::A]) || pad.pressed.left,
                    key(&[egui::Key::ArrowRight, egui::Key::D]) || pad.pressed.right,
                ),
                dy: axis(
                    key(&[egui::Key::ArrowUp, egui::Key::W]) || pad.pressed.up,
                    key(&[egui::Key::ArrowDown, egui::Key::S]) || pad.pressed.down,
                ),
                confirm: key(&[egui::Key::Enter, egui::Key::Space]) || pad.pressed.jump,
                back: key(&[egui::Key::Escape]) || pad.start_pressed,
            }
        })
    }
}

/// Moves a cursor over `count` items laid out in rows of `cols` (1 = a plain list). It stops at
/// the edges; going down from a short last row lands on the last item.
pub(in crate::app) fn move_cursor(
    cursor: usize,
    count: usize,
    cols: usize,
    dx: i32,
    dy: i32,
) -> usize {
    if count == 0 {
        return 0;
    }
    let cols = cols.max(1);
    let mut c = cursor.min(count - 1);
    if dx != 0 {
        let (row, col) = (c / cols, c % cols);
        let col = (col as i32 + dx).clamp(0, cols as i32 - 1) as usize;
        let target = row * cols + col;
        if target < count {
            c = target;
        }
    }
    if dy < 0 && c >= cols {
        c -= cols;
    } else if dy > 0 {
        if c + cols < count {
            c += cols;
        } else if c / cols < (count - 1) / cols {
            c = count - 1;
        }
    }
    c
}

/// One menu item. Returns the response (`clicked()` activates it). A mouse moving over it moves
/// the cursor too (`hover_moves`: only when the pointer actually moved, so the keyboard cursor
/// does not jump back to a resting mouse).
pub(in crate::app) fn menu_item(
    ui: &mut egui::Ui,
    text: &str,
    size: egui::Vec2,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let fill = match (enabled, selected) {
        (false, _) => ITEM_OFF,
        (true, true) => ITEM_SELECTED,
        (true, false) => ITEM,
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 6.0, fill);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(18.0),
        if enabled { TEXT } else { TEXT_OFF },
    );
    resp
}

/// Did the pointer move over `resp` in this frame (so hovering should move the cursor)?
pub(in crate::app) fn hover_moves(ui: &egui::Ui, resp: &egui::Response) -> bool {
    resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
}

/// Cursor state of the menu screens.
#[derive(Default)]
pub(in crate::app) struct MenuState {
    pub title: usize,
    /// "New game" was pressed once while a saved game exists: the next press confirms.
    pub confirm_new: bool,
    pub pause: usize,
    pub victory: usize,
    /// The cursor of the level select.
    pub level: usize,
    /// The difficulty whose high scores are shown.
    pub scores: Difficulty,
}

/// The items of the title screen, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitleItem {
    Continue,
    NewGame,
    LevelSelect,
    HighScores,
    Difficulty,
    /// Closes the window (desktop only: a web page cannot be closed by the program).
    #[cfg(not(target_arch = "wasm32"))]
    Exit,
}

const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::Continue,
    TitleItem::NewGame,
    TitleItem::LevelSelect,
    TitleItem::HighScores,
    TitleItem::Difficulty,
    #[cfg(not(target_arch = "wasm32"))]
    TitleItem::Exit,
];

impl App {
    /// Goes to the title screen with the cursor on the most useful item.
    pub(in crate::app) fn enter_title(&mut self) {
        self.screen = Screen::Title;
        self.menu.confirm_new = false;
        self.menu.title = usize::from(self.save.resume(self.cfg.difficulty).is_none());
    }

    pub(in crate::app) fn title_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let input = MenuInput::read(&ctx, &self.pad);
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, BG);

        let diff = self.cfg.difficulty;
        let has_resume = self.save.resume(diff).is_some();
        let before = self.menu.title;
        self.menu.title = move_cursor(self.menu.title, TITLE_ITEMS.len(), 1, 0, input.dy);
        // Left / right change the difficulty while it is the selected item.
        if TITLE_ITEMS[self.menu.title] == TitleItem::Difficulty && input.dx != 0 {
            self.step_difficulty(input.dx);
        }
        if self.menu.title != before {
            self.menu.confirm_new = false;
        }

        let mut activate = None;
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .id_salt("title")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space((rect.height() * 0.12).max(8.0));
        child.label(
            egui::RichText::new(t!("app.name"))
                .size(40.0)
                .strong()
                .color(TEXT),
        );
        child.add_space(24.0);
        let size = egui::vec2(ITEM_W, ITEM_H);
        for (i, item) in TITLE_ITEMS.iter().enumerate() {
            let (text, enabled) = match item {
                TitleItem::Continue => (t!("title.continue"), has_resume),
                TitleItem::NewGame if self.menu.confirm_new => (t!("title.new_game_confirm"), true),
                TitleItem::NewGame => (t!("title.new_game"), true),
                TitleItem::LevelSelect => (t!("title.levels"), true),
                TitleItem::HighScores => (t!("title.scores"), true),
                TitleItem::Difficulty => (t!("title.difficulty", name = t!(diff.key())), true),
                #[cfg(not(target_arch = "wasm32"))]
                TitleItem::Exit => (t!("menu.file.exit"), true),
            };
            let resp = menu_item(&mut child, &text, size, self.menu.title == i, enabled);
            if enabled && hover_moves(&child, &resp) && self.menu.title != i {
                self.menu.title = i;
                self.menu.confirm_new = false;
            }
            if resp.clicked() {
                self.menu.title = i;
                activate = Some(*item);
            }
            child.add_space(8.0);
        }
        let selected = TITLE_ITEMS[self.menu.title];
        if input.confirm && (selected != TitleItem::Continue || has_resume) {
            activate = Some(selected);
        }
        match activate {
            Some(TitleItem::Continue) => self.continue_game(&ctx),
            Some(TitleItem::NewGame) => {
                if has_resume && !self.menu.confirm_new {
                    self.menu.confirm_new = true;
                } else {
                    self.new_game(&ctx);
                }
            }
            Some(TitleItem::LevelSelect) => self.enter_level_select(),
            Some(TitleItem::HighScores) => self.enter_high_scores(),
            Some(TitleItem::Difficulty) => self.step_difficulty(1),
            #[cfg(not(target_arch = "wasm32"))]
            Some(TitleItem::Exit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            None => {}
        }
        // A gamepad cannot wake the app: poll it (see `game_ui`).
        let wait = if self.pad.connected { 100 } else { 1000 };
        ctx.request_repaint_after(std::time::Duration::from_millis(wait));
    }

    /// Selects the next (`dir` > 0) or previous difficulty, stopping at the ends; the cursor
    /// keeps its place (the Continue item follows the difficulty).
    pub(in crate::app) fn step_difficulty(&mut self, dir: i32) {
        let i = Difficulty::ALL
            .iter()
            .position(|&d| d == self.cfg.difficulty)
            .unwrap_or(2) as i32;
        let next = if dir > 0 && i as usize + 1 == Difficulty::ALL.len() {
            0 // a click on the last one wraps round
        } else {
            (i + dir).clamp(0, Difficulty::ALL.len() as i32 - 1)
        };
        self.cfg.difficulty = Difficulty::ALL[next as usize];
        self.cfg_dirty = true;
        self.menu.confirm_new = false;
    }

    /// The buttons of the pause menu over the field at `field`; returns the chosen one.
    pub(in crate::app) fn pause_menu(
        &mut self,
        ui: &mut egui::Ui,
        field: egui::Rect,
        input: MenuInput,
    ) -> Option<PauseChoice> {
        let items = [
            (t!("pause.resume"), PauseChoice::Resume),
            (t!("pause.levels"), PauseChoice::LevelSelect),
            (t!("pause.title"), PauseChoice::Title),
            #[cfg(not(target_arch = "wasm32"))]
            (t!("menu.file.exit"), PauseChoice::Exit),
        ];
        let size = egui::vec2(ITEM_W.min(field.width() - 16.0), ITEM_H);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field)
                .id_salt("pause_menu")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space(field.height() * 0.5 - 16.0);
        child.label(
            egui::RichText::new(t!("game.paused"))
                .size(24.0)
                .color(TEXT),
        );
        child.add_space(12.0);
        self.menu.pause = move_cursor(self.menu.pause, items.len(), 1, 0, input.dy);
        let mut chosen = None;
        for (i, (text, choice)) in items.iter().enumerate() {
            let resp = menu_item(&mut child, text, size, self.menu.pause == i, true);
            if hover_moves(&child, &resp) {
                self.menu.pause = i;
            }
            if resp.clicked() {
                chosen = Some(*choice);
            }
            child.add_space(8.0);
        }
        if input.confirm {
            chosen = Some(items[self.menu.pause].1);
        }
        chosen
    }

    /// The question after the last level: play again or quit to the title.
    pub(in crate::app) fn victory_menu(
        &mut self,
        ui: &mut egui::Ui,
        field: egui::Rect,
        input: MenuInput,
    ) -> Option<VictoryChoice> {
        let items = [
            (t!("victory.again"), VictoryChoice::PlayAgain),
            (t!("victory.quit"), VictoryChoice::Quit),
        ];
        let size = egui::vec2(ITEM_W.min(field.width() - 16.0), ITEM_H);
        ui.painter()
            .rect_filled(field, 0.0, egui::Color32::from_black_alpha(190));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field)
                .id_salt("victory_menu")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space(field.height() * 0.5 - 40.0);
        child.label(
            egui::RichText::new(t!("victory.question"))
                .size(24.0)
                .color(TEXT),
        );
        child.add_space(12.0);
        self.menu.victory = move_cursor(self.menu.victory, items.len(), 1, 0, input.dy);
        let mut chosen = None;
        for (i, (text, choice)) in items.iter().enumerate() {
            let resp = menu_item(&mut child, text, size, self.menu.victory == i, true);
            if hover_moves(&child, &resp) {
                self.menu.victory = i;
            }
            if resp.clicked() {
                chosen = Some(*choice);
            }
            child.add_space(8.0);
        }
        if input.confirm {
            chosen = Some(items[self.menu.victory].1);
        }
        chosen
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum PauseChoice {
    Resume,
    LevelSelect,
    Title,
    #[cfg(not(target_arch = "wasm32"))]
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum VictoryChoice {
    PlayAgain,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_cursor_stops_at_the_ends() {
        assert_eq!(move_cursor(0, 3, 1, 0, -1), 0);
        assert_eq!(move_cursor(0, 3, 1, 0, 1), 1);
        assert_eq!(move_cursor(2, 3, 1, 0, 1), 2);
        assert_eq!(move_cursor(1, 3, 1, 1, 0), 1);
        assert_eq!(move_cursor(0, 0, 1, 1, 1), 0);
    }

    #[test]
    fn a_grid_cursor_moves_in_rows() {
        // 7 tiles in rows of 3:  0 1 2 / 3 4 5 / 6
        assert_eq!(move_cursor(0, 7, 3, 1, 0), 1);
        assert_eq!(move_cursor(2, 7, 3, 1, 0), 2);
        assert_eq!(move_cursor(3, 7, 3, -1, 0), 3);
        assert_eq!(move_cursor(1, 7, 3, 0, 1), 4);
        assert_eq!(move_cursor(4, 7, 3, 0, 1), 6); // short last row: the last tile
        assert_eq!(move_cursor(5, 7, 3, 0, 1), 6);
        assert_eq!(move_cursor(6, 7, 3, 0, -1), 3);
        assert_eq!(move_cursor(6, 7, 3, 1, 0), 6);
        assert_eq!(move_cursor(1, 7, 3, 0, -1), 1);
        assert_eq!(move_cursor(99, 7, 3, 0, 0), 6); // a stale cursor is clamped
    }
}
