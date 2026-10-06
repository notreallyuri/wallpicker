//! The wallpaper grid and its right-click menu, shared by the Local and
//! Wallhaven tabs.

use std::collections::HashSet;

use leptos::context::Provider;
use leptos::prelude::*;
use leptos::web_sys;
use wallpicker_core::monitors::Monitor;
use wasm_bindgen::JsCast;

use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

/// Shared grid layout for both tile kinds.
pub(crate) const GRID: &str = "grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4";

/// The `data-menu-key` of the tile under a right-click, if any.
fn menu_key(e: &leptos::ev::MouseEvent) -> Option<String> {
    let el = e.target()?.dyn_into::<web_sys::Element>().ok()?;
    el.closest("[data-menu-key]")
        .ok()??
        .get_attribute("data-menu-key")
}

/// The `data-menu-key` of the tile the menu was opened on. Provided to the
/// menu's `extra` items through a `Provider`: in Leptos 0.8 a component has no
/// context scope of its own, so `provide_context` here would land on the
/// parent, where the Local and Wallhaven grids would overwrite each other's.
#[derive(Clone, Copy)]
pub(crate) struct MenuTarget(pub(crate) RwSignal<Option<String>>);

/// A grid of tiles with one right-click menu shared by all of them. Tiles mark
/// themselves with `data-menu-key`; the callbacks get that key.
#[component]
pub(crate) fn TileMenu(
    /// Apply the tile to these monitors.
    on_apply: Callback<(String, Vec<String>)>,
    /// More entries after the apply ones. They can read [`MenuTarget`].
    #[prop(optional)]
    extra: Option<ChildrenFn>,
    children: ChildrenFn,
) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let target = RwSignal::new(None::<String>);
    let extra = StoredValue::new(extra);
    // Monitors ticked under "Apply to…". Kept between right-clicks.
    let picked = RwSignal::new(HashSet::<String>::new());
    // The ticked monitors that are still connected, in the bar's order.
    let picked_targets = move || {
        picked.with(|p| {
            ctx.monitors.with(|ms| {
                ms.iter()
                    .filter(|m| p.contains(&m.name))
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>()
            })
        })
    };
    let apply = move |monitors: Vec<String>| {
        if let Some(key) = target.get_untracked() {
            on_apply.run((key, monitors));
        }
    };
    let children = StoredValue::new(children);

    view! {
        <Provider value=MenuTarget(target)>
            <ContextMenu>
                <CloseOnScroll />
                <ContextMenuTrigger>
                    <div
                        class=GRID
                        on:contextmenu=move |e| match menu_key(&e) {
                            Some(key) => {
                                target.set(Some(key));
                                // Start "Apply to…" from the bar's choice.
                                if picked_targets().is_empty() {
                                    picked.set(ctx.targets.get_untracked().into_iter().collect());
                                }
                            }
                            // Between tiles: no menu at all.
                            None => {
                                e.prevent_default();
                                e.stop_propagation();
                            }
                        }
                    >
                        {children.with_value(|c| c())}
                    </div>
                </ContextMenuTrigger>
                <ContextMenuContent>
                    <For
                        each=move || ctx.monitors.get()
                        key=|m| m.name.clone()
                        children=move |m: Monitor| {
                            let name = m.name.clone();
                            view! {
                                <ContextMenuItem on_click=move |_| apply(vec![name.clone()])>
                                    {format!("Apply to {}", m.name)}
                                </ContextMenuItem>
                            }
                        }
                    />
                    <Show when=move || ctx.monitors.with(|m| m.len() > 1)>
                        <ContextMenuItem on_click=move |_| {
                            apply(ctx.monitors.with(|ms| ms.iter().map(|m| m.name.clone()).collect()))
                        }>"Apply to all"</ContextMenuItem>
                        <ContextMenuSub>
                            <ContextMenuSubTrigger>"Apply to…"</ContextMenuSubTrigger>
                            <ContextMenuSubContent>
                                <For
                                    each=move || ctx.monitors.get()
                                    key=|m| m.name.clone()
                                    children=move |m: Monitor| {
                                        let name = m.name.clone();
                                        let checked = Signal::derive(move || picked.with(|p| p.contains(&name)));
                                        let name = m.name.clone();
                                        view! {
                                            <ContextMenuCheckboxItem
                                                checked=checked
                                                on_change=Callback::new(move |on: bool| {
                                                    picked.update(|p| {
                                                        if on {
                                                            p.insert(name.clone());
                                                        } else {
                                                            p.remove(&name);
                                                        }
                                                    })
                                                })
                                            >
                                                {m.name.clone()}
                                            </ContextMenuCheckboxItem>
                                        }
                                    }
                                />
                                <ContextMenuSeparator />
                                <ContextMenuItem
                                    disabled=Signal::derive(move || picked_targets().is_empty())
                                    on_click=move |_| apply(picked_targets())
                                >
                                    "Apply"
                                </ContextMenuItem>
                            </ContextMenuSubContent>
                        </ContextMenuSub>
                    </Show>
                    {extra.with_value(|e| e.as_ref().map(|e| e()))}
                </ContextMenuContent>
            </ContextMenu>
        </Provider>
    }
}

/// Scrolling closes the menu: it stays where the right-click was while the
/// tile it was for moves away. A scroll inside the menu itself doesn't.
#[component]
fn CloseOnScroll() -> impl IntoView {
    use wasm_bindgen::closure::Closure;
    let menu = nocomponents::primitives::context_menu::use_context_menu();
    let on_scroll = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| {
        if !menu.is_open.get_untracked() {
            return;
        }
        let inside = menu
            .content_ref
            .get_untracked()
            .zip(e.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()))
            .is_some_and(|(content, target)| content.contains(Some(&target)));
        if !inside {
            menu.is_open.set(false);
        }
    });
    // Scroll events don't bubble: listen while they go down (capture), so
    // any scroll area counts. The grids live as long as the window, so the
    // listener does too.
    let _ = window().add_event_listener_with_callback_and_bool("scroll", on_scroll.as_ref().unchecked_ref(), true);
    on_scroll.forget();
}
