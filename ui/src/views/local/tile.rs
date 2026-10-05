//! A local wallpaper: click to apply, star to favorite.

use std::path::PathBuf;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wallpicker_core::library::LocalImage;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

#[component]
pub(super) fn LocalTile(img: LocalImage) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let src = RwSignal::new(None::<String>);
    let broken = RwSignal::new(false);
    let path = img.path.to_string_lossy().into_owned();

    {
        let path = path.clone();
        spawn_local(async move {
            match api::thumbnail(&path).await {
                Ok(thumb) => {
                    src.try_set(Some(api::asset_url(&thumb)));
                }
                Err(_) => {
                    broken.try_set(true);
                }
            }
        });
    }

    let img_path = img.path.clone();
    let shown_on = move || {
        ctx.state.with(|s| {
            s.current
                .iter()
                .filter(|(_, p)| **p == img_path)
                .map(|(m, _)| m.clone())
                .collect::<Vec<_>>()
        })
    };
    let busy_path = path.clone();
    let busy = move || {
        ctx.applying
            .with(|a| a.as_deref() == Some(busy_path.as_str()))
    };
    let key: StoredValue<PathBuf> = StoredValue::new(img.path.clone());
    let favorite = Signal::derive(move || ctx.catalog.with(|c| key.with_value(|p| c.is_favorite(p))));
    let tags = move || ctx.catalog.with(|c| key.with_value(|p| c.tags(p).to_vec()));
    let click_path = path.clone();
    let star_path = path.clone();
    let menu_key = path.clone();
    let delete_path = path.clone();
    let trash = expect_context::<super::trash::TrashConfirm>();
    // Its size, and the monitors a click applies to that are bigger than it
    // (it would be scaled up there). Only a hint: nothing is hidden.
    let size = (img.width > 0).then(|| format!("{}×{}", img.width, img.height));
    let sized = img.clone();
    let too_small_for = Memo::new(move |_| {
        let targets = ctx.targets.get();
        ctx.monitors.with(|ms| {
            ms.iter()
                .filter(|m| targets.contains(&m.name) && sized.smaller_than(m.width, m.height))
                .map(|m| format!("{}'s {}×{}", m.name, m.width, m.height))
                .collect::<Vec<_>>()
        })
    });

    view! {
        // The star is a sibling of the tile's button, not inside it: buttons
        // can't contain buttons.
        <div data-menu-key=menu_key class="group relative">
            <button
                type="button"
                title=path
                on:click=move |_| spawn_local(ctx.apply(click_path.clone()))
                // Delete asks to move it to the trash, as the menu does.
                on:keydown=move |e| {
                    if e.key() == "Delete" {
                        e.prevent_default();
                        trash.ask(delete_path.clone());
                    }
                }
                class="flex w-full flex-col overflow-hidden rounded-3xl bg-card/80 p-1.5 text-left shadow-md shadow-black/10 ring-1 ring-foreground/5 transition duration-300 ease-bounce hover:-translate-y-1 hover:shadow-lg hover:shadow-primary/20 hover:ring-2 hover:ring-primary/70 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            >
                <div class="relative aspect-[16/10] w-full overflow-hidden rounded-[1.1rem] bg-muted">
                    {move || match (src.get(), broken.get()) {
                        (Some(s), _) => {
                            view! {
                                <img
                                    src=s
                                    alt=""
                                    class="size-full object-cover transition duration-300 group-hover:scale-[1.03]"
                                />
                            }
                                .into_any()
                        }
                        (None, true) => {
                            view! {
                                <div class="flex size-full items-center justify-center text-xs text-muted-foreground">
                                    "No preview"
                                </div>
                            }
                                .into_any()
                        }
                        (None, false) => {
                            view! { <Skeleton class="size-full rounded-none" /> }.into_any()
                        }
                    }} <div class="absolute top-2 left-2 flex flex-wrap gap-1">
                        <For
                            each=shown_on
                            key=|m| m.clone()
                            children=|m: String| view! { <Badge>{m}</Badge> }
                        />
                    </div> <Show when=busy.clone()>
                        <div class="absolute inset-0 flex items-center justify-center bg-background/60">
                            <Spinner />
                        </div>
                    </Show>
                </div>
                <div class="flex min-w-0 flex-col gap-1 px-3 py-2 text-xs">
                    <div class="flex min-w-0 items-baseline justify-between gap-2">
                        <span class="truncate text-muted-foreground">{img.name.clone()}</span>
                        {size.map(|size| {
                            view! {
                                <span
                                    class=move || {
                                        if too_small_for.with(Vec::is_empty) {
                                            "shrink-0 text-[10px] text-muted-foreground/70"
                                        } else {
                                            "shrink-0 text-[10px] text-amber-500"
                                        }
                                    }
                                    title=move || {
                                        let small = too_small_for.get();
                                        if small.is_empty() {
                                            String::new()
                                        } else {
                                            format!("Smaller than {} (it would be scaled up)", small.join(", "))
                                        }
                                    }
                                >
                                    {size}
                                </span>
                            }
                        })}
                    </div>
                    <Show when=move || !tags().is_empty()>
                        <div class="flex gap-1 overflow-hidden">
                            <For
                                each=tags
                                key=|t| t.clone()
                                children=|t: String| {
                                    view! {
                                        <span class="shrink-0 rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-medium text-primary">
                                            {t}
                                        </span>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </button>
            <button
                type="button"
                aria-label=move || if favorite.get() { "Remove from favorites" } else { "Add to favorites" }
                title=move || if favorite.get() { "Remove from favorites" } else { "Add to favorites" }
                aria-pressed=move || favorite.get().to_string()
                on:click=move |_| spawn_local(ctx.set_favorite(star_path.clone(), !favorite.get_untracked()))
                class=move || {
                    format!(
                        "absolute top-2 right-2 flex size-7 items-center justify-center rounded-full bg-background/80 transition hover:bg-background focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-ring {}",
                        if favorite.get() {
                            "text-yellow-400"
                        } else {
                            "text-foreground/80 opacity-0 group-hover:opacity-100"
                        },
                    )
                }
            >
                <StarIcon filled=favorite />
            </button>
        </div>
    }
}

/// Lucide's star.
#[component]
pub(super) fn StarIcon(#[prop(into)] filled: Signal<bool>) -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            class="size-4"
            fill=move || if filled.get() { "currentColor" } else { "none" }
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z" />
        </svg>
    }
}
