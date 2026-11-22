//! Zoomable tool windows (#32, taken from MM PortLens `native_window.rs`): one window with its own
//! OS window (egui immediate viewport) that can be moved outside the main window and onto another
//! monitor, or - when `Config::separate_windows` is off or the backend has no native windows
//! (`Context::embed_viewports`, always the case in the browser) - an `egui::Window` inside the
//! main window.
//!
//! Both variants have their own text zoom (Ctrl + wheel, Ctrl + `+` / `-` / `0`, the bottom bar),
//! independent of the main window: the text styles of the window's `Ui` are scaled, because
//! `Context::set_zoom_factor` belongs to the context all viewports share. Position, size and zoom
//! are kept in a `config::ToolWindowState`. Used by About and Settings.

use crate::app::*;
use crate::config::ToolWindowState;

/// Smallest and largest zoom of a tool window, step of a button / key press.
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 3.0;
const ZOOM_STEP: f32 = 0.1;

/// The fixed look of a tool window.
pub(in crate::app) struct ToolSpec {
    pub title: String,
    /// Inner size of the first window (logical pixels) and its minimum.
    pub default_size: [f32; 2],
    pub min_size: [f32; 2],
}

/// One tool window: open or not, and the geometry its native window opened with.
pub(in crate::app) struct ToolWindow {
    /// Unique key: the viewport id and the egui id are derived from it.
    key: &'static str,
    open: bool,
    /// Bring the window to the front in the next frame.
    focus: bool,
    /// Position and size the native window was opened with. Fixed at its first frame: a builder
    /// that changes between frames moves the open window.
    opened_at: Option<(Option<[f32; 2]>, [f32; 2])>,
}

impl ToolWindow {
    pub(in crate::app) const fn new(key: &'static str) -> Self {
        Self {
            key,
            open: false,
            focus: false,
            opened_at: None,
        }
    }

    pub(in crate::app) fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the window, or brings the open one to the front.
    pub(in crate::app) fn open(&mut self) {
        self.open = true;
        self.focus = true;
    }

    pub(in crate::app) fn close(&mut self) {
        self.open = false;
        self.focus = false;
        self.opened_at = None;
    }

    /// Draws the window when it is open: the bottom zoom bar and `content` (a central panel).
    /// `state` gets the zoom and, for a native window, the position and size; Esc (native), the
    /// close button and Close close the window.
    pub(in crate::app) fn show(
        &mut self,
        ctx: &egui::Context,
        native: bool,
        spec: &ToolSpec,
        state: &mut ToolWindowState,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        if !self.open {
            return;
        }
        state.zoom = clamp_zoom(state.zoom);
        let close = if native {
            self.show_native(ctx, spec, state, content)
        } else {
            self.opened_at = None;
            self.show_embedded(ctx, spec, state, content)
        };
        if close {
            self.close();
        }
    }

    fn show_native(
        &mut self,
        ctx: &egui::Context,
        spec: &ToolSpec,
        state: &mut ToolWindowState,
        content: impl FnOnce(&mut egui::Ui),
    ) -> bool {
        let (pos, size) = *self.opened_at.get_or_insert_with(|| {
            let size = state
                .size
                .map(|[w, h]| [w.max(spec.min_size[0]), h.max(spec.min_size[1])])
                .unwrap_or(spec.default_size);
            (state.pos, size)
        });
        let mut builder = egui::ViewportBuilder::default()
            .with_title(&spec.title)
            .with_inner_size(size)
            .with_min_inner_size(spec.min_size);
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.with_icon(std::sync::Arc::new(egui::IconData {
                rgba: crate::icon::rgba(64),
                width: 64,
                height: 64,
            }));
        }
        if let Some(pos) = pos {
            builder = builder.with_position(pos);
        }
        let viewport = egui::ViewportId::from_hash_of(self.key);
        if self.focus {
            self.focus = false;
            ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Focus);
        }
        let mut content = Some(content);
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            let ctx = ui.ctx().clone();
            let mut zoom = state.zoom;
            zoom_input(&ctx, &mut zoom, true);
            scale_text(ui.style_mut(), zoom);
            let mut close = false;
            egui::Panel::bottom("tool_window_bar").show(ui, |ui| close |= zoom_bar(ui, &mut zoom));
            egui::CentralPanel::default().show(ui, |ui| {
                if let Some(c) = content.take() {
                    c(ui);
                }
            });
            // Not inside `input`: the context is locked there.
            let ui_zoom = ctx.zoom_factor();
            let (close_requested, esc, geometry) = ctx.input(|i| {
                let vp = i.viewport();
                (
                    vp.close_requested(),
                    i.key_pressed(egui::Key::Escape),
                    window_geometry(vp, ui_zoom),
                )
            });
            if let Some((pos, size)) = geometry {
                state.pos = pos;
                state.size = Some(size);
            }
            if zoom != state.zoom {
                state.zoom = zoom;
                ctx.request_repaint();
            }
            close || close_requested || esc
        })
    }

    fn show_embedded(
        &mut self,
        ctx: &egui::Context,
        spec: &ToolSpec,
        state: &mut ToolWindowState,
        content: impl FnOnce(&mut egui::Ui),
    ) -> bool {
        let id = egui::Id::unique(self.key);
        if self.focus {
            self.focus = false;
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
        }
        let mut open = true;
        let mut close = false;
        let mut zoom = state.zoom;
        egui::Window::new(&spec.title)
            .id(id)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(spec.default_size)
            .min_size(spec.min_size)
            .show(ctx, |ui| {
                // The zoom keys and the wheel belong to this window while the pointer is over it.
                let over = ui.rect_contains_pointer(ui.max_rect());
                zoom_input(ctx, &mut zoom, over);
                scale_text(ui.style_mut(), zoom);
                egui::Panel::bottom("tool_window_bar")
                    .show(ui, |ui| close |= zoom_bar(ui, &mut zoom));
                egui::CentralPanel::default().show(ui, content);
            });
        if zoom != state.zoom {
            state.zoom = zoom;
            ctx.request_repaint();
        }
        close || !open
    }
}

/// A zoom inside the limits, rounded to whole percent (so ten steps return exactly to 100 %).
pub(in crate::app) fn clamp_zoom(z: f32) -> f32 {
    if !z.is_finite() {
        return 1.0;
    }
    (z.clamp(MIN_ZOOM, MAX_ZOOM) * 100.0).round() / 100.0
}

/// The position and size of a native window in logical pixels (egui points include the zoom);
/// `None` while minimized / maximized or before the size is known.
fn window_geometry(vp: &egui::ViewportInfo, zoom: f32) -> Option<(Option<[f32; 2]>, [f32; 2])> {
    if vp.minimized == Some(true) || vp.maximized == Some(true) {
        return None;
    }
    let inner = vp.inner_rect?;
    Some((
        vp.outer_rect.map(|r| [r.min.x * zoom, r.min.y * zoom]),
        [inner.width() * zoom, inner.height() * zoom],
    ))
}

/// Ctrl + wheel and Ctrl + `+` / `-` / `0` of this frame applied to `zoom` (when `active`); the
/// zoom keys are consumed, so egui's own keyboard zoom of the whole program does not see them.
fn zoom_input(ctx: &egui::Context, zoom: &mut f32, active: bool) {
    if !active {
        return;
    }
    let (wheel, (plus, minus, reset)) = ctx.input_mut(|i| {
        let keys = zoom_keys(&i.events);
        if keys != (false, false, false) {
            i.events
                .retain(|e| zoom_keys(std::slice::from_ref(e)) == (false, false, false));
        }
        (i.zoom_delta(), keys)
    });
    let mut z = *zoom;
    if wheel != 1.0 {
        z *= wheel;
    }
    if plus {
        z += ZOOM_STEP;
    }
    if minus {
        z -= ZOOM_STEP;
    }
    if reset {
        z = 1.0;
    }
    *zoom = clamp_zoom(z);
}

/// The bottom bar: A- / zoom % (click = 100 %) / A+, the hint and Close. Changes `zoom`; returns
/// true when Close was clicked.
fn zoom_bar(ui: &mut egui::Ui, zoom: &mut f32) -> bool {
    let mut close = false;
    let mut z = *zoom;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("A-")
            .on_hover_text(t!("toolwin.zoom_out"))
            .clicked()
        {
            z -= ZOOM_STEP;
        }
        let pct = format!("{:.0} %", z * 100.0);
        if ui
            .button(egui::RichText::new(pct).monospace())
            .on_hover_text(t!("toolwin.zoom_reset"))
            .clicked()
        {
            z = 1.0;
        }
        if ui
            .button("A+")
            .on_hover_text(t!("toolwin.zoom_in"))
            .clicked()
        {
            z += ZOOM_STEP;
        }
        // The hint gives way (shortened) when the zoomed buttons and Close need the room.
        let hint_width = (ui.available_width() - 90.0).max(0.0);
        ui.scope(|ui| {
            ui.set_max_width(hint_width);
            ui.add(
                egui::Label::new(egui::RichText::new(t!("toolwin.zoom_hint")).weak()).truncate(),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(t!("toolwin.close")).clicked() {
                close = true;
            }
        });
    });
    ui.add_space(4.0);
    *zoom = clamp_zoom(z);
    close
}

/// The zoom keys pressed in this frame: (larger, smaller, reset). Ctrl with `+` / `=`, `-`, `0`
/// (also on the numeric keypad); Shift is ignored (`+` needs it on many layouts) and the physical
/// key counts too, so Ctrl + 0 works where the key gives another character.
fn zoom_keys(events: &[egui::Event]) -> (bool, bool, bool) {
    use egui::Key;
    let (mut plus, mut minus, mut reset) = (false, false, false);
    for e in events {
        let egui::Event::Key {
            key,
            physical_key,
            pressed: true,
            modifiers,
            ..
        } = e
        else {
            continue;
        };
        if !modifiers.command || modifiers.alt {
            continue;
        }
        let is = |k: Key| *key == k || *physical_key == Some(k);
        plus |= is(Key::Plus) || is(Key::Equals);
        minus |= is(Key::Minus);
        reset |= is(Key::Num0);
    }
    (plus, minus, reset)
}

/// Scales the text styles of `style` by `zoom`.
fn scale_text(style: &mut egui::Style, zoom: f32) {
    if zoom != 1.0 {
        for font in style.text_styles.values_mut() {
            font.size *= zoom;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_in_logical_pixels() {
        let vp = egui::ViewportInfo {
            outer_rect: Some(egui::Rect::from_min_size(
                egui::pos2(100.0, 50.0),
                egui::vec2(410.0, 330.0),
            )),
            inner_rect: Some(egui::Rect::from_min_size(
                egui::pos2(105.0, 80.0),
                egui::vec2(400.0, 300.0),
            )),
            ..Default::default()
        };
        let (pos, size) = window_geometry(&vp, 1.5).unwrap();
        assert_eq!(pos, Some([150.0, 75.0]));
        assert_eq!(size, [600.0, 450.0]);
        let min = egui::ViewportInfo {
            minimized: Some(true),
            ..vp
        };
        assert_eq!(window_geometry(&min, 1.0), None);
    }

    #[test]
    fn open_focus_close() {
        let mut w = ToolWindow::new("test_tool");
        assert!(!w.is_open());
        w.open();
        assert!(w.is_open() && w.focus);
        w.opened_at = Some((None, [1.0, 1.0]));
        // Opening again only focuses.
        w.focus = false;
        w.open();
        assert!(w.focus && w.opened_at.is_some());
        // Closing forgets the opening geometry, so the next one uses the remembered state.
        w.close();
        assert!(!w.is_open() && w.opened_at.is_none());
    }

    #[test]
    fn zoom_is_clamped_and_rounded() {
        assert_eq!(clamp_zoom(f32::NAN), 1.0);
        assert_eq!(clamp_zoom(10.0), MAX_ZOOM);
        assert_eq!(clamp_zoom(0.1), MIN_ZOOM);
        let mut z = 1.0;
        for _ in 0..10 {
            z = clamp_zoom(z + ZOOM_STEP);
        }
        for _ in 0..10 {
            z = clamp_zoom(z - ZOOM_STEP);
        }
        assert_eq!(z, 1.0);
    }
}
