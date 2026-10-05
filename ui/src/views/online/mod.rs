//! The Online tab: search a provider (Wallhaven, Konachan, yande.re), download and
//! apply. Each provider brings its own filters; the results, pager, tiles and
//! details dialog are shared.

pub(crate) mod collections;
mod details;
mod moebooru;
mod pager;
mod sizes;
mod tile;
mod wallhaven;

use std::collections::HashSet;

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::icons::search::Search as SearchIcon;
use nocomponents::primitives::tabs::use_tabs;
use wallpicker_core::moebooru::{self as booru, MoebooruQuery, TagSuggestion};
use wallpicker_core::online::{OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider};
use wallpicker_core::wallhaven::SearchQuery;

use crate::api;
use crate::app::TAB_ONLINE;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::tile_menu::{MenuTarget, TileMenu};
use crate::views::Choice;

use details::DetailsDialog;
use moebooru::{MoebooruFilters, MoebooruForm, ratings_apply};
use pager::Pager;
use tile::{RemoteTile, RemoteTiles};
use wallhaven::{WallhavenFilters, WallhavenForm};

/// Minimum sizes offered by every provider's filters.
const MIN_RES: &[(&str, &str)] = &wallpicker_core::online::MIN_RESOLUTIONS;

/// `(value, label)` pairs for a [`crate::views::Choice`].
fn pairs<T: Copy>(
    items: &[(T, &'static str)],
    as_str: fn(T) -> &'static str,
) -> Vec<(&'static str, &'static str)> {
    items.iter().map(|(v, l)| (as_str(*v), *l)).collect()
}

/// Shared by the grid's menu, the tiles and the details dialog.
#[derive(Clone, Copy)]
pub(super) struct Online {
    results: RwSignal<Vec<OnlineWallpaper>>,
    /// The wallpaper the details dialog shows.
    details: RwSignal<Option<OnlineWallpaper>>,
    details_open: RwSignal<bool>,
    /// Search the current provider for this query (a tag, an uploader),
    /// newest first. The form shows it.
    search: Callback<String>,
}

impl Online {
    fn wallpaper(self, key: &str) -> Option<OnlineWallpaper> {
        self.results
            .with_untracked(|r| r.iter().find(|w| w.key() == key).cloned())
    }

    fn show_details(self, w: OnlineWallpaper) {
        self.details.set(Some(w));
        self.details_open.set(true);
    }
}

/// A new page of results starts at the top, not where the last one was scrolled to.
fn scroll_to_top() {
    if let Ok(Some(el)) = document().query_selector("[data-scroll-root]") {
        el.set_scroll_top(0);
    }
}

fn placeholder(p: Provider) -> &'static str {
    match p {
        Provider::Wallhaven => "Search Wallhaven: tags, +include -exclude, @uploader, id:<tag id>",
        Provider::Konachan => "Search Konachan: tags, comma-separated (long hair, night) or long_hair night; -tag to exclude",
        Provider::Yandere => "Search yande.re: tags, comma-separated (long hair, night) or long_hair night; -tag to exclude",
    }
}

/// Lucide's "funnel".
#[component]
fn FilterIcon() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            class="size-4"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z" />
        </svg>
    }
}

/// The online-only entries of the grid's right-click menu.
#[component]
fn RemoteMenuItems() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tiles = expect_context::<RemoteTiles>();
    let online = expect_context::<Online>();
    let MenuTarget(target) = expect_context::<MenuTarget>();
    let current = move || target.get_untracked().and_then(|key| online.wallpaper(&key));

    view! {
        <ContextMenuSeparator />
        <ContextMenuItem on_click=move |_| {
            if let Some(w) = current() {
                online.show_details(w);
            }
        }>"Details…"</ContextMenuItem>
        // Already downloaded: Apply (above) is all there is to do.
        <Show when=move || !target.with(|t| t.as_deref().is_some_and(|k| tiles.is_saved(k)))>
            <ContextMenuSeparator />
            <ContextMenuItem on_click=move |_| {
                if let Some(w) = current() {
                    tiles.fetch(ctx, w, None);
                }
            }>"Download"</ContextMenuItem>
        </Show>
    }
}

#[component]
pub(crate) fn OnlineView() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tabs = use_tabs();

    // A select rather than a button row: the list of providers grows.
    let provider_choice = RwSignal::new(Provider::default().as_str().to_string());
    let provider =
        Memo::new(move |_| provider_choice.with(|p| Provider::from_str(p).unwrap_or_default()));

    // Search text is shared, so a term can be tried on each provider.
    let text = RwSignal::new(String::new());
    let wallhaven = WallhavenForm::new();
    let konachan = MoebooruForm::new();
    let yandere = MoebooruForm::new();

    let results = RwSignal::new(Vec::<OnlineWallpaper>::new());
    let tiles = RemoteTiles {
        working: RwSignal::new(HashSet::new()),
        saved: RwSignal::new(HashSet::new()),
    };
    provide_context(tiles);
    // Which results are already downloaded. Reloaded after downloads and
    // settings changes (which may move the download folder); both bump
    // `library_rev`.
    Effect::new(move |_| {
        ctx.library_rev.track();
        spawn_local(async move {
            match api::online_downloaded().await {
                Ok(keys) => {
                    tiles.saved.set(keys.into_iter().collect());
                }
                Err(e) => leptos::logging::error!("Could not list downloaded wallpapers: {e}"),
            }
        });
    });
    // Paging info of the shown results (its `items` stay empty).
    let page_info = RwSignal::new(None::<Page>);
    let loading = RwSignal::new(false);

    let build_query = move || {
        untrack(|| match provider.get() {
            Provider::Wallhaven => {
                let mut q = wallhaven.filters();
                q.q = text.get();
                // The backend drops this without an API key anyway.
                q.purity.nsfw &= ctx.has_api_key();
                OnlineQuery::Wallhaven(q)
            }
            Provider::Konachan => {
                let mut q = konachan.filters();
                q.tags = text.get();
                OnlineQuery::Konachan(q)
            }
            Provider::Yandere => {
                let mut q = yandere.filters();
                q.tags = text.get();
                OnlineQuery::Yandere(q)
            }
        })
    };

    // Bring back last session's provider and filters, then save them
    // whenever they change. Nothing is saved (or searched) before they're
    // back, so the defaults never overwrite the saved file.
    let restored = RwSignal::new(false);
    spawn_local(async move {
        match api::get_online_filters().await {
            Ok(f) => {
                provider_choice.set(f.provider.as_str().to_string());
                wallhaven.restore(f.wallhaven);
                konachan.restore(f.konachan);
                yandere.restore(f.yandere);
            }
            Err(e) => ctx.error_with("Could not load the saved search filters", &e),
        }
        restored.set(true);
    });
    Effect::new(move |_| {
        let f = OnlineFilters {
            provider: provider.get(),
            wallhaven: wallhaven.filters(),
            konachan: konachan.filters(),
            yandere: yandere.filters(),
        };
        if restored.get_untracked() {
            spawn_local(async move {
                if let Err(e) = api::save_online_filters(&f).await {
                    leptos::logging::error!("Could not save the search filters: {e}");
                }
            });
        }
    });

    // The query behind the shown results. Pages re-run it, so editing the
    // form without searching doesn't change what page 2 means.
    let last_query = StoredValue::new(None::<OnlineQuery>);
    // Only the newest request's answer is shown: switching provider while a
    // search runs must not let the old answer land afterwards.
    let latest = StoredValue::new(0u64);
    let searched_with = StoredValue::new(None::<Provider>);

    let fetch = move |q: OnlineQuery| {
        let id = latest.get_value() + 1;
        latest.set_value(id);
        searched_with.set_value(Some(q.provider()));
        loading.set(true);
        spawn_local(async move {
            let result = api::online_search(&q).await;
            if latest.get_value() != id {
                return;
            }
            match result {
                Ok(mut page) => {
                    // Keys must be unique; random pages can repeat an image.
                    let mut seen = HashSet::new();
                    results.set(
                        std::mem::take(&mut page.items)
                            .into_iter()
                            .filter(|w| seen.insert(w.key()))
                            .collect(),
                    );
                    page_info.set(Some(page));
                    last_query.set_value(Some(q));
                    scroll_to_top();
                }
                Err(e) => {
                    let name = q.provider().label();
                    ctx.error_with(&format!("{name} search failed"), &e);
                }
            }
            loading.set(false);
        });
    };
    // A new search from the form, starting at page 1.
    let run = move || fetch(build_query());
    let go_to = move |page: u64| {
        let Some(q) = last_query.get_value() else {
            return;
        };
        // Random sorting needs the seed to keep its order across pages.
        let seed = page_info.with_untracked(|p| p.as_ref().and_then(|p| p.seed.clone()));
        fetch(q.at_page(u32::try_from(page).unwrap_or(u32::MAX), seed));
    };

    let online = Online {
        results,
        details: RwSignal::new(None),
        details_open: RwSignal::new(false),
        search: Callback::new(move |q: String| {
            text.set(q);
            match provider.get_untracked() {
                Provider::Wallhaven => wallhaven.sort_latest(),
                Provider::Konachan => konachan.sort_latest(),
                Provider::Yandere => yandere.sort_latest(),
            }
            run();
        }),
    };
    provide_context(online);

    // Search when the tab is first opened (not at app start), and whenever
    // the provider changes.
    Effect::new(move |_| {
        let p = provider.get();
        if restored.get()
            && tabs.current_tab.get() == TAB_ONLINE
            && searched_with.get_value() != Some(p)
        {
            run();
        }
    });

    // A Wallhaven search asked for from the Collections tab (a tag, an
    // uploader). Changing the provider searches by itself; otherwise run it.
    Effect::new(move |_| {
        let Some(q) = ctx.online_search.get() else {
            return;
        };
        ctx.online_search.set(None);
        text.set(q);
        wallhaven.sort_latest();
        if provider.get_untracked() == Provider::Wallhaven {
            run();
        } else {
            provider_choice.set(Provider::Wallhaven.as_str().to_string());
        }
    });

    // Konachan and yande.re match exact tag names only: the real tags what's
    // typed may mean, looked up a moment after typing stops.
    let suggestions = RwSignal::new(Vec::<TagSuggestion>::new());
    let suggest_seq = StoredValue::new(0u64);
    Effect::new(move |_| {
        let typed = text.get();
        let p = provider.get();
        let n = suggest_seq.get_value() + 1;
        suggest_seq.set_value(n);
        if p == Provider::Wallhaven || booru::plain_words(&typed).is_empty() {
            suggestions.set(Vec::new());
            return;
        }
        set_timeout(
            move || {
                if suggest_seq.get_value() != n {
                    return;
                }
                spawn_local(async move {
                    let got = api::tag_suggestions(p, &typed).await.unwrap_or_default();
                    if suggest_seq.get_value() != n {
                        return;
                    }
                    // Nothing to suggest once the last tag is a real one.
                    let last = booru::plain_words(&typed).last().cloned();
                    let exact = got.iter().any(|s| !s.replaces_all && Some(&s.name) == last.as_ref());
                    suggestions.set(if exact { Vec::new() } else { got });
                });
            },
            std::time::Duration::from_millis(350),
        );
    });
    let pick_suggestion = move |s: TagSuggestion| {
        text.set(booru::apply_suggestion(&text.get_untracked(), &s));
        run();
    };

    let filters_open = RwSignal::new(false);
    // Filters that differ from the defaults, shown on the Filters button.
    let active_filters = move || {
        let moebooru = |p: Provider, form: MoebooruForm| {
            let mut f = form.filters();
            // Ratings that don't apply (konachan.net) don't count either.
            if !ratings_apply(ctx, p) {
                f.ratings = MoebooruQuery::default().ratings;
            }
            f.active_filters()
        };
        match provider.get() {
            Provider::Wallhaven => wallhaven.filters().active_filters(),
            Provider::Konachan => moebooru(Provider::Konachan, konachan),
            Provider::Yandere => moebooru(Provider::Yandere, yandere),
        }
    };

    // (current page, last page), once there are results.
    let pages = move || page_info.with(|p| p.as_ref().map(|p| (p.page, p.last_page)));
    let searched = move || page_info.with(Option::is_some);

    view! {
        // Stays at the top while the results scroll.
        <form
            class="sticky top-0 z-20 -mx-5 -mt-5 border-b border-border/60 bg-background/70 px-5 pt-5 pb-3 backdrop-blur-xl mb-5 flex flex-wrap items-center gap-2"
            on:submit=move |ev| {
                ev.prevent_default();
                run();
            }
        >
            <Choice
                items=Provider::ALL.iter().map(|p| (p.as_str(), p.label())).collect()
                model=provider_choice
                class="w-36"
            />
            <div class="relative min-w-64 flex-1">
                <SearchIcon class="pointer-events-none absolute top-2 left-2.5 text-muted-foreground" />
                <Input
                    model=text
                    class="pl-8"
                    attr:placeholder=move || placeholder(provider.get())
                    attr:data-shortcut="search"
                />
            </div>
            <Button
                variant=ButtonVariant::Outline
                attr:title="Sorting and filters"
                on_click=move |_| filters_open.set(true)
            >
                <FilterIcon />
                "Filters"
                <Show when=move || { active_filters() > 0 }>
                    <span class="rounded-full bg-primary px-1.5 text-xs leading-5 text-primary-foreground">
                        {active_filters}
                    </span>
                </Show>
            </Button>
            <Button r#type=ButtonType::Submit disabled=loading>
                "Search"
            </Button>
            <Show when=move || suggestions.with(|s| !s.is_empty())>
                <div class="flex basis-full flex-wrap items-center gap-1.5 text-xs">
                    <span class="text-muted-foreground">"Tags:"</span>
                    <For
                        each=move || suggestions.get()
                        key=|s| s.name.clone()
                        children=move |s: TagSuggestion| {
                            let label = format!("{}  {}", s.name, s.count);
                            let kind = s.kind.clone();
                            let title = format!("{kind} tag");
                            let note = (kind != "general").then_some(kind);
                            view! {
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::Xs
                                    class="rounded-full"
                                    attr:title=title
                                    on_click=move |_| pick_suggestion(s.clone())
                                >
                                    {label.clone()}
                                    {note.clone().map(|k| view! { <span class="text-muted-foreground">{k}</span> })}
                                </Button>
                            }
                        }
                    />
                </div>
            </Show>
        </form>

        // Sorting and filters live in a dialog, so the results start right
        // under the search box.
        <Dialog open=filters_open>
            <DialogPortal class="max-h-[90vh] overflow-y-auto sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>{move || format!("{} filters", provider.get().label())}</DialogTitle>
                    <DialogDescription>
                        "Kept as you change them, and used by the next search."
                    </DialogDescription>
                </DialogHeader>
                <div class="flex flex-col gap-5">
                    {move || match provider.get() {
                        Provider::Wallhaven => view! { <WallhavenFilters form=wallhaven /> }.into_any(),
                        Provider::Konachan => {
                            view! { <MoebooruFilters form=konachan provider=Provider::Konachan /> }
                                .into_any()
                        }
                        Provider::Yandere => {
                            view! { <MoebooruFilters form=yandere provider=Provider::Yandere /> }
                                .into_any()
                        }
                    }}
                </div>
                <DialogFooter class="flex gap-2">
                    <Button
                        variant=ButtonVariant::Ghost
                        class="mr-auto"
                        on_click=move |_| match provider.get_untracked() {
                            Provider::Wallhaven => wallhaven.restore(SearchQuery::default()),
                            Provider::Konachan => konachan.restore(MoebooruQuery::default()),
                            Provider::Yandere => yandere.restore(MoebooruQuery::default()),
                        }
                    >
                        "Reset"
                    </Button>
                    <Button on_click=move |_| {
                        filters_open.set(false);
                        run();
                    }>"Search"</Button>
                </DialogFooter>
            </DialogPortal>
        </Dialog>

        <Show when=move || searched() && !loading.get() && results.with(|r| r.is_empty())>
            <Empty>
                <EmptyHeader>
                    <EmptyTitle>"Nothing found"</EmptyTitle>
                    <EmptyDescription>"No wallpapers match these filters."</EmptyDescription>
                </EmptyHeader>
            </Empty>
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
                {move || {
                    page_info
                        .with(|p| {
                            p.as_ref()
                                .map(|p| {
                                    // Konachan's count can be missing; then
                                    // only the page is known.
                                    if p.total > 0 {
                                        format!("{} results · page {} of {}", p.total, p.page, p.last_page)
                                    } else {
                                        format!("page {}", p.page)
                                    }
                                })
                        })
                }}
            </div>
        </div>

        <DetailsDialog />
    }
}
