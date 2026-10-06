//! The details dialog of an online wallpaper: a bigger preview, the uploader,
//! the source and the tags, each of which can start a new search.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wallpicker_core::online::{Details, OnlineTag, OnlineWallpaper};
use wallpicker_core::wallhaven::is_openable_url;

use super::Online;
use super::tile::{RemoteTiles, likes_label};
use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::components::tip::Tip;

/// Details of the wallpaper with this key, once loaded.
type Loaded = Option<(String, Result<Details, String>)>;

#[component]
pub(super) fn DetailsDialog() -> impl IntoView {
    let online = expect_context::<Online>();
    let info = RwSignal::new(None as Loaded);

    Effect::new(move |_| {
        let Some(w) = online.details.get() else {
            return;
        };
        let key = w.key();
        // Some providers include the details in the search result.
        if let Some(d) = w.details.clone() {
            info.set(Some((key, Ok(d))));
            return;
        }
        // Reopening the same wallpaper reuses what we have (Wallhaven allows
        // 45 requests a minute).
        if info.with_untracked(|i| matches!(i, Some((k, Ok(_))) if *k == key)) {
            return;
        }
        info.set(None);
        spawn_local(async move {
            let result = api::online_details(&w).await;
            // Drop the answer if another wallpaper was opened meanwhile.
            let still_shown = online
                .details
                .with_untracked(|d| d.as_ref().is_some_and(|d| d.key() == key));
            if still_shown {
                info.try_set(Some((key, result)));
            }
        });
    });

    view! {
        <Dialog open=online.details_open>
            // A wallpaper can have dozens of tags: cap the height and scroll.
            <DialogPortal class="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
                {move || online.details.get().map(|w| view! { <DetailsBody w=w info=info /> })}
            </DialogPortal>
        </Dialog>
    }
}

#[component]
fn DetailsBody(w: OnlineWallpaper, info: RwSignal<Loaded>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tiles = expect_context::<RemoteTiles>();
    let online = expect_context::<Online>();

    let wallpaper = StoredValue::new(w.clone());
    let close = move || online.details_open.set(false);
    // Searches replace the results behind the dialog, so close it.
    let search = move |q: String| {
        close();
        online.search.run(q);
    };
    let open = move |url: String| {
        spawn_local(async move {
            if let Err(e) = api::open_url(&url).await {
                ctx.error_with("Could not open the link", &e);
            }
        });
    };
    let key = w.key();
    let working = Signal::derive({
        let key = key.clone();
        move || tiles.working.with(|s| s.contains(&key))
    });
    let saved = Signal::derive({
        let key = key.clone();
        move || tiles.is_saved(&key)
    });
    let page = w.page_url.clone();
    let provider = w.provider;

    let about = move || match info.get().filter(|(k, _)| *k == key).map(|(_, r)| r) {
        None => view! {
            <div class="flex items-center gap-2 text-muted-foreground">
                <Spinner />
                "Loading tags…"
            </div>
        }
        .into_any(),
        Some(Err(e)) => view! {
            <p class="text-muted-foreground">{format!("Could not load the details: {e}")}</p>
        }
        .into_any(),
        Some(Ok(d)) => {
            let source = Some(d.source).filter(|s| is_openable_url(s));
            let (artists, tags): (Vec<OnlineTag>, Vec<OnlineTag>) =
                d.tags.into_iter().partition(|t| t.is_artist);
            view! {
                <div class="flex flex-col gap-3">
                    {artists
                        .into_iter()
                        .map(|a| {
                            let q = a.query.clone();
                            view! {
                                <div class="flex flex-wrap items-center gap-2">
                                    <span class="text-muted-foreground">"Artist"</span>
                                    <span class="font-medium">{a.name.clone()}</span>
                                    <Button size=ButtonSize::Xs on_click=move |_| search(q.clone())>
                                        {format!("More by {}", a.name)}
                                    </Button>
                                </div>
                            }
                        })
                        .collect_view()}
                    <div class="flex flex-wrap items-center gap-2">
                        {d
                            .uploader
                            .map(|u| {
                                view! {
                                    <span class="text-muted-foreground">"Uploaded by"</span>
                                    <span class="font-medium">{u.name.clone()}</span>
                                    {u
                                        .query
                                        .map(|q| {
                                            view! {
                                                <Button
                                                    size=ButtonSize::Xs
                                                    variant=ButtonVariant::Outline
                                                    on_click=move |_| search(q.clone())
                                                >
                                                    {format!("More from {}", u.name)}
                                                </Button>
                                            }
                                        })}
                                }
                            })}
                        {source
                            .map(|url| {
                                let title = url.clone();
                                view! {
                                    <Tip text=title>
                                        {
                                            let url = url.clone();
                                            view! {
                                                <Button
                                                    size=ButtonSize::Xs
                                                    variant=ButtonVariant::Outline
                                                    on_click=move |_| open(url.clone())
                                                >
                                                    "Open source"
                                                </Button>
                                            }
                                        }
                                    </Tip>
                                }
                            })}
                    </div>
                    <div class="flex flex-wrap gap-1.5">
                        {if tags.is_empty() {
                            view! { <span class="text-muted-foreground">"No tags."</span> }
                                .into_any()
                        } else {
                            tags.into_iter()
                                .map(|t| {
                                    let q = t.query.clone();
                                    let name = t.name.clone();
                                    view! {
                                        // The category, where the provider has one,
                                        // shows on hover.
                                        <Tip text=t.category.clone()>
                                            {
                                                let (q, name) = (q.clone(), name.clone());
                                                view! {
                                                    <Button
                                                        size=ButtonSize::Xs
                                                        variant=ButtonVariant::Secondary
                                                        on_click=move |_| search(q.clone())
                                                    >
                                                        {name.clone()}
                                                    </Button>
                                                }
                                            }
                                        </Tip>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </div>
            }
            .into_any()
        }
    };

    let mut stats = vec![likes_label(&w)];
    stats.extend(w.views.map(|v| format!("{v} views")));
    if w.file_size > 0 {
        stats.push(format!("{:.1} MB", w.file_size as f64 / 1_000_000.0));
    }
    let title = format!("{} · {}", w.resolution(), w.category);
    let preview = w.preview.clone();
    // The sharp image (Wallhaven's original) fades in over the quick one
    // once it has loaded; Moebooru's preview already is the sharp one.
    let sharp = Some(w.details_image().to_string()).filter(|s| *s != preview && !s.is_empty());
    let sharp_loaded = RwSignal::new(false);

    view! {
        <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>
                {stats.join(" · ")} {move || saved.get().then_some(" · Downloaded")}
            </DialogDescription>
        </DialogHeader>
        <div class="relative aspect-[16/10] max-h-[45vh] w-full overflow-hidden rounded-lg bg-muted">
            <img src=preview alt="" class="absolute inset-0 size-full object-cover" />
            {sharp
                .map(|src| {
                    view! {
                        <img
                            src=src
                            alt=""
                            on:load=move |_| sharp_loaded.set(true)
                            class=move || {
                                format!(
                                    "absolute inset-0 size-full object-cover transition-opacity duration-300 {}",
                                    if sharp_loaded.get() { "opacity-100" } else { "opacity-0" },
                                )
                            }
                        />
                    }
                })}
        </div>
        {about}
        <DialogFooter class="flex flex-wrap gap-2">
            {is_openable_url(&page)
                .then(|| {
                    view! {
                        <Button
                            variant=ButtonVariant::Ghost
                            class="mr-auto"
                            on_click=move |_| open(page.clone())
                        >
                            {format!("Open on {}", provider.label())}
                        </Button>
                    }
                })}
            <Show when=move || !saved.get()>
                <Button
                    variant=ButtonVariant::Secondary
                    disabled=working
                    on_click=move |_| tiles.fetch(ctx, wallpaper.get_value(), None)
                >
                    "Download"
                </Button>
            </Show>
            <Button
                disabled=working
                on_click=move |_| {
                    close();
                    tiles.fetch(ctx, wallpaper.get_value(), Some(ctx.targets.get_untracked()));
                }
            >
                "Apply"
            </Button>
        </DialogFooter>
    }
}
