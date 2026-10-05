//! Tab headers float over their scroll area (`data-tab-head`, absolutely
//! positioned), so the grid scrolls under their blur. They're outside the
//! scrolling element, so unlike a sticky header they can't lag behind a fast
//! scroll. The scroll area makes room for its header through `--head-h`
//! (padding), kept up to date here as the header grows (tags expanded) or the
//! tab is shown; and `--sb-w`, its scrollbar's width, keeps the header off
//! the scrollbar.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlElement, ResizeObserver, ResizeObserverEntry};

fn set_var(pane: &HtmlElement, name: &str, px: f64) {
    let _ = pane.style().set_property(name, &format!("{px}px"));
}

fn measure(el: &web_sys::Element) {
    let Some(pane) = el.parent_element().and_then(|p| p.dyn_into::<HtmlElement>().ok()) else { return };
    if el.has_attribute("data-tab-head") {
        let height = el.get_bounding_client_rect().height();
        // A hidden tab measures 0; keep what it had.
        if height > 0.0 {
            set_var(&pane, "--head-h", height);
        }
    } else if let Some(scroller) = el.dyn_ref::<HtmlElement>()
        && scroller.offset_width() > 0
    {
        set_var(&pane, "--sb-w", f64::from(scroller.offset_width() - scroller.client_width()));
    }
}

/// Watch every tab's header and scroll area. Once, after the app is mounted.
pub(crate) fn install() {
    let on_resize = Closure::<dyn Fn(js_sys::Array)>::new(|entries: js_sys::Array| {
        for entry in entries.iter() {
            measure(&entry.unchecked_into::<ResizeObserverEntry>().target());
        }
    });
    let Ok(observer) = ResizeObserver::new(on_resize.as_ref().unchecked_ref()) else { return };
    // Lives as long as the app.
    on_resize.forget();
    if let Ok(found) = document().query_selector_all("[data-tab-head], [data-scroll-root]") {
        for i in 0..found.length() {
            if let Some(el) = found.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                observer.observe(&el);
            }
        }
    }
}
