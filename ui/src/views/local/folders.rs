//! The Folders layout: a card per subfolder, and the way back up.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wallpicker_core::library::{self, Place, Subfolder};

use crate::api;
use crate::components::nc::prelude::*;

/// A subfolder: its first wallpaper as the cover, its name and how many are
/// in it (or below). Opens on click.
#[component]
pub(super) fn FolderCard(sub: Subfolder, cover: String, on_open: Callback<Option<Place>>) -> impl IntoView {
    let src = RwSignal::new(None::<String>);
    spawn_local(async move {
        if let Ok(thumb) = api::thumbnail(&cover).await {
            src.try_set(Some(api::asset_url(&thumb)));
        }
    });
    let place = StoredValue::new(sub.place.clone());
    view! {
        <button
            type="button"
            title=sub.name.clone()
            on:click=move |_| on_open.run(Some(place.get_value()))
            class="group flex w-full flex-col overflow-hidden rounded-3xl bg-card/80 p-1.5 text-left shadow-md shadow-black/10 ring-1 ring-foreground/5 transition duration-300 ease-bounce hover:-translate-y-1 hover:shadow-lg hover:shadow-primary/20 hover:ring-2 hover:ring-primary/70 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        >
            <div class="relative aspect-[16/10] w-full overflow-hidden rounded-[1.1rem] bg-muted">
                {move || {
                    src.get()
                        .map(|s| {
                            view! { <img src=s alt="" class="size-full object-cover opacity-50 transition group-hover:opacity-70" /> }
                        })
                }}
                <div class="absolute inset-0 flex items-center justify-center text-foreground">
                    <Lucide class="size-10 drop-shadow">
                        <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                    </Lucide>
                </div>
            </div>
            <div class="flex min-w-0 items-center justify-between gap-2 px-3 py-2 text-xs">
                <span class="truncate font-medium">{sub.name.clone()}</span>
                <span class="shrink-0 text-muted-foreground">{sub.count}</span>
            </div>
        </button>
    }
}

/// Up, then the way here (each step clickable), and "Searching in … and
/// below" while a filter is on.
#[component]
pub(super) fn Breadcrumb(
    place: RwSignal<Option<Place>>,
    #[prop(into)] root_names: Signal<Vec<String>>,
    #[prop(into)] filtering: Signal<bool>,
) -> impl IntoView {
    let roots = move || root_names.with(Vec::len);
    let up = move || library::parent_place(place.get().as_ref(), roots());
    let steps = move || root_names.with(|names| library::trail(place.get().as_ref(), names, "All folders"));
    view! {
        <div class="flex flex-wrap items-center gap-1 text-sm">
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::IconSm
                attr:aria-label="Up a folder"
                attr:title="Up a folder (Alt+Up)"
                attr:data-shortcut="folder-up"
                disabled=Signal::derive(move || up().is_none())
                on_click=move |_| {
                    if let Some(to) = up() {
                        place.set(to);
                    }
                }
            >
                <Lucide class="size-4">
                    <polyline points="14 9 9 4 4 9" />
                    <path d="M20 20h-7a4 4 0 0 1-4-4V4" />
                </Lucide>
            </Button>
            {move || {
                let all = steps();
                let last = all.len().saturating_sub(1);
                all.into_iter()
                    .enumerate()
                    .map(|(i, (label, to))| {
                        let current = i == last;
                        view! {
                            {(i > 0).then(|| view! { <span class="text-muted-foreground">"›"</span> })}
                            <Button
                                variant=if current { ButtonVariant::Secondary } else { ButtonVariant::Ghost }
                                size=ButtonSize::Sm
                                on_click=move |_| {
                                    if !current {
                                        place.set(to.clone());
                                    }
                                }
                            >
                                {label.clone()}
                            </Button>
                        }
                    })
                    .collect_view()
            }}
            <Show when=move || filtering.get()>
                <span class="ml-2 text-muted-foreground">
                    {move || {
                        let here = steps().last().map(|(l, _)| l.clone()).unwrap_or_default();
                        format!("Searching in {here} and below")
                    }}
                </span>
            </Show>
        </div>
    }
}

/// A Lucide icon's frame; the shapes go inside.
#[component]
pub(super) fn Lucide(#[prop(into)] class: String, children: Children) -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            class=class
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            {children()}
        </svg>
    }
}
