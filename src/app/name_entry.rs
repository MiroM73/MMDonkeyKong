//! Entering the name for a new high score, arcade style: three letters, up / down change the
//! letter under the cursor, left / right move it, Enter / A / the OK button go on. Keyboard (typing
//! a letter works too), gamepad and touch (arrow buttons) all work. The state machine is pure.

use crate::app::screens::{ITEM_H, menu_item};
use crate::app::*;
use crate::save::NAME_LEN;

const LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const TEXT: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);

/// What happens once the name is entered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum EntryNext {
    /// After a game over: the level starts again.
    Restart,
    /// After the last level: the "play again / quit" question.
    Victory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) struct NameEntry {
    /// Index into `LETTERS` of every position.
    letters: [u8; NAME_LEN],
    pos: usize,
    pub score: u32,
    pub next: EntryNext,
}

impl NameEntry {
    /// Starts with the letters of `last_name` (anything else becomes "A").
    pub fn new(score: u32, next: EntryNext, last_name: &str) -> Self {
        let mut letters = [0u8; NAME_LEN];
        for (slot, c) in letters.iter_mut().zip(last_name.chars()) {
            if let Some(i) = LETTERS
                .iter()
                .position(|&l| char::from(l) == c.to_ascii_uppercase())
            {
                *slot = i as u8;
            }
        }
        Self {
            letters,
            pos: 0,
            score,
            next,
        }
    }

    pub fn name(&self) -> String {
        self.letters
            .iter()
            .map(|&i| char::from(LETTERS[usize::from(i)]))
            .collect()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn up(&mut self) {
        let l = &mut self.letters[self.pos];
        *l = (*l + 1) % LETTERS.len() as u8;
    }

    pub fn down(&mut self) {
        let l = &mut self.letters[self.pos];
        *l = (*l + LETTERS.len() as u8 - 1) % LETTERS.len() as u8;
    }

    pub fn left(&mut self) {
        self.pos = self.pos.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.pos = (self.pos + 1).min(NAME_LEN - 1);
    }

    pub fn select(&mut self, pos: usize) {
        self.pos = pos.min(NAME_LEN - 1);
    }

    /// A typed character: a letter is set and the cursor moves on; anything else is ignored.
    pub fn type_char(&mut self, c: char) {
        if let Some(i) = LETTERS
            .iter()
            .position(|&l| char::from(l) == c.to_ascii_uppercase())
        {
            self.letters[self.pos] = i as u8;
            self.right();
        }
    }

    pub fn backspace(&mut self) {
        self.left();
    }

    /// Enter / A: on to the next letter, and finished after the last one.
    pub fn confirm(&mut self) -> bool {
        if self.pos + 1 < NAME_LEN {
            self.pos += 1;
            false
        } else {
            true
        }
    }
}

impl App {
    /// A game ended with `score` on the way to `next`: asks for a name when the score makes the
    /// table, otherwise goes straight on.
    pub(in crate::app) fn finish_run(&mut self, ctx: &egui::Context, score: u32, next: EntryNext) {
        let d = self.game.state.difficulty;
        if self.save.progress_mut(d).qualifies(score) {
            self.game.entry = Some(NameEntry::new(score, next, &self.cfg.last_name));
        } else {
            self.continue_after(ctx, next);
        }
    }

    /// The name was entered (or skipped): store the score and go on.
    fn submit_name(&mut self, ctx: &egui::Context, entry: NameEntry) {
        let d = self.game.state.difficulty;
        let name = entry.name();
        self.save.add_high_score(d, &name, entry.score);
        self.cfg.last_name = name;
        self.cfg_dirty = true;
        self.persist();
        self.game.entry = None;
        self.continue_after(ctx, entry.next);
    }

    fn continue_after(&mut self, ctx: &egui::Context, next: EntryNext) {
        match next {
            EntryNext::Restart => {
                let (d, level) = (self.game.state.difficulty, self.game.level);
                self.save.game_over(d, level, d.params().lives);
                self.persist();
                self.begin_level(ctx, level, None, 0);
            }
            EntryNext::Victory => self.game.victory_menu = true,
        }
    }

    /// Draws and runs the name entry over the field at `field`.
    pub(in crate::app) fn name_entry_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        field: egui::Rect,
    ) {
        let Some(mut entry) = self.game.entry.take() else {
            return;
        };
        // Keyboard (arrows only: W / A / S / D are letters here), typed letters, gamepad.
        let (typed, keys) = ctx.input(|i| {
            let mut typed = Vec::new();
            for e in &i.events {
                if let egui::Event::Text(s) = e {
                    typed.extend(s.chars());
                }
            }
            let key = |k: egui::Key| i.key_pressed(k);
            (
                typed,
                [
                    key(egui::Key::ArrowUp),
                    key(egui::Key::ArrowDown),
                    key(egui::Key::ArrowLeft),
                    key(egui::Key::ArrowRight),
                    key(egui::Key::Backspace),
                    key(egui::Key::Enter),
                ],
            )
        });
        let pad = self.pad.pressed;
        let [k_up, k_down, k_left, k_right, k_back, k_enter] = keys;
        let mut done = false;
        for c in typed {
            entry.type_char(c);
        }
        if k_up || pad.up {
            entry.up();
        }
        if k_down || pad.down {
            entry.down();
        }
        if k_left || pad.left {
            entry.left();
        }
        if k_right || pad.right {
            entry.right();
        }
        if k_back {
            entry.backspace();
        }
        if k_enter || pad.jump {
            done = entry.confirm();
        }

        ui.painter()
            .rect_filled(field, 0.0, egui::Color32::from_black_alpha(200));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field)
                .id_salt("name_entry")
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.add_space((field.height() * 0.5 - 120.0).max(8.0));
        child.label(
            egui::RichText::new(t!("entry.title"))
                .size(26.0)
                .color(TEXT),
        );
        child.label(
            egui::RichText::new(t!("entry.score", score = entry.score))
                .size(18.0)
                .color(TEXT),
        );
        child.add_space(10.0);
        let cell = egui::vec2(56.0, 44.0);
        let name = entry.name();
        let width = NAME_LEN as f32 * (cell.x + 10.0) - 10.0;
        let pad_left = ((child.available_width() - width) / 2.0).max(0.0);
        let mut taps = Vec::new();
        child.horizontal(|ui| {
            ui.add_space(pad_left);
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 6.0);
            for (i, ch) in name.chars().enumerate() {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);
                    if menu_item(ui, "▲", egui::vec2(cell.x, 32.0), false, true).clicked() {
                        taps.push((i, 1));
                    }
                    if menu_item(ui, &ch.to_string(), cell, entry.pos() == i, true).clicked() {
                        taps.push((i, 0));
                    }
                    if menu_item(ui, "▼", egui::vec2(cell.x, 32.0), false, true).clicked() {
                        taps.push((i, -1));
                    }
                });
            }
        });
        for (i, dir) in taps {
            entry.select(i);
            match dir {
                1 => entry.up(),
                -1 => entry.down(),
                _ => {}
            }
        }
        child.add_space(14.0);
        let ok = egui::vec2(220.0_f32.min(field.width() - 16.0), ITEM_H);
        if menu_item(&mut child, &t!("entry.ok"), ok, false, true).clicked() {
            done = true;
        }
        child.add_space(6.0);
        child.label(egui::RichText::new(t!("entry.hint")).size(13.0).color(TEXT));

        if done {
            self.submit_name(ctx, entry);
        } else {
            self.game.entry = Some(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> NameEntry {
        NameEntry::new(100, EntryNext::Restart, "")
    }

    #[test]
    fn starts_with_the_last_name_or_aaa() {
        assert_eq!(entry().name(), "AAA");
        assert_eq!(NameEntry::new(1, EntryNext::Restart, "mir").name(), "MIR");
        assert_eq!(NameEntry::new(1, EntryNext::Restart, "x1").name(), "XAA");
        assert_eq!(NameEntry::new(1, EntryNext::Restart, "ťažký").name(), "AAA");
    }

    #[test]
    fn up_and_down_wrap_around_the_alphabet() {
        let mut e = entry();
        e.down();
        assert_eq!(e.name(), "ZAA");
        e.up();
        e.up();
        assert_eq!(e.name(), "BAA");
    }

    #[test]
    fn the_cursor_stops_at_the_ends() {
        let mut e = entry();
        e.left();
        assert_eq!(e.pos(), 0);
        for _ in 0..5 {
            e.right();
        }
        assert_eq!(e.pos(), NAME_LEN - 1);
    }

    #[test]
    fn typing_sets_the_letter_and_moves_on() {
        let mut e = entry();
        for c in "m1r!i".chars() {
            e.type_char(c);
        }
        // "m" "r" "i": the digit and the "!" are ignored, the last letter stays on the last place.
        assert_eq!(e.name(), "MRI");
        e.backspace();
        e.type_char('z');
        assert_eq!(e.name(), "MZI");
    }

    #[test]
    fn confirm_goes_on_and_finishes_after_the_last_letter() {
        let mut e = entry();
        assert!(!e.confirm());
        assert!(!e.confirm());
        assert_eq!(e.pos(), NAME_LEN - 1);
        assert!(e.confirm());
    }
}
