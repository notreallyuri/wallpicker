//! A small popover of wallpicker's own (nocomponents' dropdown had traps):
//! `Popover` wraps a trigger and a `PopoverPanel`, and closes `open` on a
//! click outside it or Esc. The trigger flips `open` itself.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub(crate) fn Popover(open: RwSignal<bool>, children: Children) -> impl IntoView {
    let root = NodeRef::<html::Div>::new();
    let _ = window_event_listener(ev::pointerdown, move |e| {
        if !open.get_untracked() {
            return;
        }
        let inside = root.get_untracked().zip(e.target()).is_some_and(|(el, t)| {
            t.dyn_ref::<web_sys::Node>().is_some_and(|n| el.contains(Some(n)))
        });
        if !inside {
            open.set(false);
        }
    });
    let _ = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    view! { <div node_ref=root class="relative">{children()}</div> }
}

/// The panel, under the trigger while `open`: from its left edge, or with
/// `end` from its right edge (for a trigger at the end of a row).
#[component]
pub(crate) fn PopoverPanel(
    open: RwSignal<bool>,
    #[prop(optional)] end: bool,
    #[prop(optional, into)] class: String,
    children: ChildrenFn,
) -> impl IntoView {
    let class = format!(
        "absolute top-full z-30 mt-2 rounded-2xl border-2 bg-popover shadow-xl {} {class}",
        if end { "right-0" } else { "left-0" },
    );
    let children = StoredValue::new(children);
    view! {
        <Show when=move || open.get()>
            <div class=class.clone()>{children.with_value(|c| c())}</div>
        </Show>
    }
}
