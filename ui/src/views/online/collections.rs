//! The Collections tab: your Wallhaven collections (private ones too, with
//! the API key), shown like search results: apply, download, details. Opening
//! one needs your Wallhaven username (Settings → Wallhaven), since the API
//! doesn't say whose key it is.

use std::collections::HashSet;

use leptos::context::Provider;
use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::primitives::tabs::use_tabs;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use wallpicker_core::online::{OnlineWallpaper, Page};
use wallpicker_core::wallhaven::Collection;

use super::details::DetailsDialog;
use super::pager::Pager;
use super::tile::{RemoteTile, RemoteTiles};
use super::{Online, RemoteMenuItems, scroll_to_top};
use crate::api;
use crate::app::{TAB_COLLECTIONS, TAB_ONLINE};
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::tile_menu::TileMenu;

#[component]
pub(crate) fn CollectionsView() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tabs = use_tabs();

    // Your collections: None while loading.
    let list = RwSignal::new(None::<Result<Vec<Collection>, String>>);
    // The one shown (a single-select ToggleGroup's value: its id).
    let chosen = RwSignal::new(Vec::<String>::new());
    let results = RwSignal::new(Vec::<OnlineWallpaper>::new());
    let page_info = RwSignal::new(None::<Page>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let tiles = RemoteTiles {
        working: RwSignal::new(HashSet::new()),
        saved: RwSignal::new(HashSet::new()),
    };
    Effect::new(move |_| {
        ctx.library_rev.track();
        spawn_local(async move {
            if let Ok(keys) = api::online_downloaded().await {
                tiles.saved.set(keys.into_iter().collect());
            }
        });
    });
    // A tag or uploader in a wallpaper's details: searched on the Online tab.
    let online = Online {
        results,
        details: RwSignal::new(None),
        details_open: RwSignal::new(false),
        search: Callback::new(move |q: String| {
            ctx.online_search.set(Some(q));
            tabs.current_tab.set(TAB_ONLINE.to_string());
        }),
    };

    let load_list = move || {
        list.set(None);
        spawn_local(async move {
            let got = api::wallhaven_collections().await;
            // Keep the choice if it still exists, else the first one.
            if let Ok(cs) = &got {
                let still = chosen.with_untracked(|c| c.first().is_some_and(|id| cs.iter().any(|x| x.id.to_string() == *id)));
                if !still {
                    chosen.set(cs.first().map(|c| vec![c.id.to_string()]).unwrap_or_default());
                }
            }
            list.try_set(Some(got));
        });
    };
    // Loaded when the tab is first opened, and again when the key or the
    // username may have changed (Settings saves bump `library_rev`).
    Effect::new(move |prev: Option<bool>| {
        ctx.library_rev.track();
        let open = tabs.current_tab.get() == TAB_COLLECTIONS;
        if open {
            load_list();
        }
        prev.unwrap_or(false) || open
    });

    let latest = StoredValue::new(0u64);
    let fetch = move |id: u64, page: u32| {
        let n = latest.get_value() + 1;
        latest.set_value(n);
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            let got = api::wallhaven_collection(id, page).await;
            if latest.get_value() != n {
                return;
            }
            match got {
                Ok(mut p) => {
                    let mut seen = HashSet::new();
                    results.set(std::mem::take(&mut p.items).into_iter().filter(|w| seen.insert(w.key())).collect());
                    page_info.set(Some(p));
                    scroll_to_top("collections");
                }
                Err(e) => {
                    results.set(Vec::new());
                    page_info.set(None);
                    error.set(Some(e));
                }
            }
            loading.set(false);
        });
    };
    let chosen_id = move || chosen.with(|c| c.first().and_then(|id| id.parse::<u64>().ok()));
    // A collection picked (or a new list): its first page.
    Effect::new(move |_| {
        if let Some(id) = chosen_id() {
            fetch(id, 1);
        }
    });
    // Clicking the selected one again would clear the choice: keep one.
    Effect::new(move |prev: Option<Vec<String>>| {
        let now = chosen.get();
        match prev {
            Some(prev) if now.is_empty() && !prev.is_empty() => {
                chosen.set(prev.clone());
                prev
            }
            _ => now,
        }
    });
    let pages = move || page_info.with(|p| p.as_ref().map(|p| (p.page, p.last_page)));
    let go_to = move |page: u64| {
        if let Some(id) = chosen_id() {
            fetch(id, u32::try_from(page).unwrap_or(u32::MAX));
        }
    };

    view! {
        <Provider value=tiles>
            <Provider value=online>
                // Stays at the top while the results scroll.
                <div data-tab-head class="absolute top-3 left-3 right-[calc(var(--sb-w,0px)+0.75rem)] z-20 rounded-2xl border border-border/60 bg-background/70 shadow-lg shadow-black/15 flex flex-wrap items-center gap-2 px-4 pt-2.5 pb-2 backdrop-blur-xl">
                    <span class="text-sm text-muted-foreground">"Wallhaven collections"</span>
                    {move || match list.get() {
                        None => view! { <Spinner /> }.into_any(),
                        Some(Err(e)) => view! { <span class="text-sm text-destructive">{e}</span> }.into_any(),
                        Some(Ok(cs)) if cs.is_empty() => {
                            view! { <span class="text-sm text-muted-foreground">"You have no collections yet."</span> }.into_any()
                        }
                        Some(Ok(cs)) => view! {
                            <ToggleGroup
                                kind=ToggleGroupType::Single
                                variant=ToggleVariant::Outline
                                size=ToggleSize::Sm
                                value=chosen
                                class="flex-wrap gap-1.5"
                            >
                                {cs
                                    .into_iter()
                                    .map(|c| {
                                        view! {
                                            <ToggleGroupItem value=c.id.to_string() class="rounded-full! px-3">
                                                {c.label.clone()}
                                                <span class="text-muted-foreground">{c.count}</span>
                                                {(!c.public).then(|| view! { <span class="text-[10px] text-muted-foreground">"private"</span> })}
                                            </ToggleGroupItem>
                                        }
                                    })
                                    .collect_view()}
                            </ToggleGroup>
                        }
                        .into_any(),
                    }}
                    <Button class="ml-auto" variant=ButtonVariant::Outline on_click=move |_| load_list()>
                        "Refresh"
                    </Button>
                </div>

                <div data-scroll-root="collections" class="relative min-h-0 flex-1 scroll-pt-[calc(var(--head-h,0px)+0.75rem)] overflow-y-auto px-5 pb-5 pt-[calc(var(--head-h,0px)+1.75rem)]">
                <Show when=move || error.with(Option::is_some)>
                    <p class="text-sm text-destructive">{move || error.get()}</p>
                </Show>

                <TileMenu
                    on_apply=Callback::new(move |(key, monitors): (String, Vec<String>)| {
                        if let Some(w) = online.wallpaper(&key) {
                            tiles.fetch(ctx, w, Some(monitors));
                        }
                    })
                    extra=std::sync::Arc::new(|| view! { <RemoteMenuItems /> }.into_any())
                >
                    <For
                        each=move || results.get()
                        key=|w| w.key()
                        children=|w: OnlineWallpaper| view! { <RemoteTile w=w /> }
                    />
                </TileMenu>

                <div class="mt-6 flex flex-col items-center gap-3">
                    <Show when=move || pages().is_some_and(|(_, last)| last > 1)>
                        <Pager pages=pages loading=loading go_to=Callback::new(go_to) />
                    </Show>
                    <div class="flex items-center gap-2 text-xs text-muted-foreground">
                        <Show when=move || loading.get()>
                            <Spinner />
                        </Show>
                        {move || page_info.with(|p| p.as_ref().map(|p| format!("{} wallpapers · page {} of {}", p.total, p.page, p.last_page)))}
                    </div>
                </div>
                </div>

                <DetailsDialog />
            </Provider>
        </Provider>
    }
}
