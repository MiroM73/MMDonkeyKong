//! Browser only: the "update available" hint of the installable web app. `index.html` registers
//! the service worker (`web/sw.js`); when a new build is waiting it fires the window event
//! `mm-update-ready` and sets `window.mmUpdateReady`; `window.mmApplyUpdate()` switches to the new
//! worker and reloads the page.

use std::cell::Cell;
use std::rc::Rc;

use web_sys::js_sys::{Function, Reflect};
use web_sys::wasm_bindgen::{JsCast, JsValue, closure::Closure};

use crate::app::*;

/// Watches for a waiting new build. Event driven: the listener repaints once, nothing polls.
pub(in crate::app) struct UpdateWatch {
    ready: Rc<Cell<bool>>,
    /// Kept alive for as long as the listener is registered.
    _listener: Option<Closure<dyn FnMut()>>,
}

impl UpdateWatch {
    pub(in crate::app) fn new(ctx: &egui::Context) -> Self {
        let ready = Rc::new(Cell::new(false));
        let Some(window) = web_sys::window() else {
            return Self {
                ready,
                _listener: None,
            };
        };
        // The event may have fired before the app started.
        let flag = Reflect::get(&window, &JsValue::from_str("mmUpdateReady")).unwrap_or_default();
        ready.set(flag.is_truthy());
        let (r, ctx) = (ready.clone(), ctx.clone());
        let listener = Closure::<dyn FnMut()>::new(move || {
            r.set(true);
            ctx.request_repaint();
        });
        let _ = window
            .add_event_listener_with_callback("mm-update-ready", listener.as_ref().unchecked_ref());
        Self {
            ready,
            _listener: Some(listener),
        }
    }

    fn apply(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Ok(f) = Reflect::get(&window, &JsValue::from_str("mmApplyUpdate"))
            && let Some(f) = f.dyn_ref::<Function>()
        {
            let _ = f.call0(&window);
        }
    }
}

impl App {
    /// A button at the right end of the menu bar while a new version is waiting.
    pub(in crate::app) fn update_hint(&mut self, ui: &mut egui::Ui) {
        if !self.update_watch.ready.get() {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = egui::Button::new(t!("update.available")).fill(egui::Color32::DARK_GREEN);
            if ui.add(button).on_hover_text(t!("update.hover")).clicked() {
                self.update_watch.apply();
            }
        });
    }
}
