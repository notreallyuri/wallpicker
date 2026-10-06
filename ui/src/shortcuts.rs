//! Window-wide keyboard shortcuts. Each one presses the control it stands for
//! (found by `data-shortcut`, its label, or its role), so it does exactly
//! what a click would, and nothing when that control isn't on screen. They
//! are off while a dialog is open.
//!
//! Ctrl+F search field · Ctrl+R random · Alt+Up parent folder ·
//! Alt+Left/Right online pages · Ctrl+1/2/3 Local/Online/Collections

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

/// The elements matching `selector` that are on screen (hidden tab panels'
/// aren't), in document order.
fn on_screen(selector: &str) -> Vec<HtmlElement> {
    let Ok(list) = document().query_selector_all(selector) else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i))
        .filter_map(|n| n.dyn_into::<HtmlElement>().ok())
        .filter(|el| el.offset_parent().is_some())
        .collect()
}

fn press(selector: &str) {
    if let Some(el) = on_screen(selector).into_iter().next() {
        el.click();
    }
}

/// No browser right-click menu (Back, Reload, Inspect…): wallpicker has its
/// own where they mean something. Text fields keep theirs (cut, copy,
/// paste). Elements with their own menus still get the event first.
fn no_browser_menu() {
    let _ = window_event_listener(ev::contextmenu, |e: ev::MouseEvent| {
        let in_text = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| el.closest("input, textarea, [contenteditable]").ok().flatten().is_some());
        if !in_text {
            e.prevent_default();
        }
    });
}

pub(crate) fn install() {
    no_browser_menu();
    // Lives as long as the window: never removed.
    let _ = window_event_listener(ev::keydown, |e: ev::KeyboardEvent| {
        if !on_screen("[role=dialog]").is_empty() {
            return;
        }
        let ctrl = e.ctrl_key() || e.meta_key();
        let alt = e.alt_key();
        let key = e.key();
        let handled = match (ctrl, alt, key.as_str()) {
            (true, false, "f" | "F") => {
                if let Some(el) = on_screen("[data-shortcut=search]").into_iter().next() {
                    let _ = el.focus();
                }
                true
            }
            // Also keeps the webview from reloading.
            (true, false, "r" | "R") => {
                press("[data-shortcut=random]");
                true
            }
            (false, true, "ArrowUp") => {
                press("[data-shortcut=folder-up]:not([disabled])");
                true
            }
            (false, true, "ArrowLeft") => {
                press("[aria-label='Go to previous page']");
                true
            }
            (false, true, "ArrowRight") => {
                press("[aria-label='Go to next page']");
                true
            }
            (true, false, "1" | "2" | "3") => {
                let n = key.parse::<usize>().unwrap_or(1) - 1;
                if let Some(tab) = on_screen("[role=tab]").into_iter().nth(n) {
                    tab.click();
                }
                true
            }
            _ => false,
        };
        if handled {
            e.prevent_default();
        }
    });
}
