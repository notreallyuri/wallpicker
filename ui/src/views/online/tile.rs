//! An online search result, and the download/apply state shared by all of
//! them.

use std::collections::HashSet;

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::icons::check::Check;
use wallpicker_core::online::{OnlineWallpaper, Provider};

use super::Online;
use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::components::tip::Tip;

/// Download state of online wallpapers, by [`OnlineWallpaper::key`]. Kept
/// above the tiles so the grid's right-click menu and the details dialog
/// share it.
#[derive(Clone, Copy)]
pub(super) struct RemoteTiles {
    pub(super) working: RwSignal<HashSet<String>>,
    /// Already in the download folder: loaded from there, and added to as
    /// downloads finish. These only get "Apply", not "Download".
    pub(super) saved: RwSignal<HashSet<String>>,
}

impl RemoteTiles {
    pub(super) fn is_saved(self, key: &str) -> bool {
        self.saved.with(|s| s.contains(key))
    }

    /// Download `w`, then apply it to `apply_to`, or just report where it was
    /// saved when that is `None`.
    pub(super) fn fetch(self, ctx: Ctx, w: OnlineWallpaper, apply_to: Option<Vec<String>>) {
        let key = w.key();
        if !self
            .working
            .try_update(|s| s.insert(key.clone()))
            .unwrap_or(false)
        {
            return; // already in progress
        }
        spawn_local(async move {
            let result = api::online_download(&w).await;
            self.working.try_update(|s| s.remove(&key));
            match result {
                Ok(path) => {
                    self.saved.try_update(|s| s.insert(key));
                    match apply_to {
                        Some(targets) => ctx.apply_to(path, targets).await,
                        None => ctx.info(format!("Saved {path}")),
                    }
                    // Show the new file in the Local tab.
                    ctx.library_rev.update(|r| *r += 1);
                    // The download may have added artist tags.
                    ctx.refresh_catalog().await;
                }
                Err(e) => ctx.error_with("Download failed", &e),
            }
        });
    }
}

/// "♥ 12" for Wallhaven favorites, "Score 19" on the Moebooru sites.
pub(super) fn likes_label(w: &OnlineWallpaper) -> String {
    match w.provider {
        Provider::Wallhaven => format!("♥ {}", w.likes),
        Provider::Konachan | Provider::Yandere => format!("Score {}", w.likes),
    }
}

#[component]
pub(super) fn RemoteTile(w: OnlineWallpaper) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tiles = expect_context::<RemoteTiles>();
    let online = expect_context::<Online>();
    let key = w.key();
    let working = Signal::derive({
        let key = key.clone();
        move || tiles.working.with(|s| s.contains(&key))
    });
    let saved = Signal::derive({
        let key = key.clone();
        move || tiles.is_saved(&key)
    });

    let wallpaper = StoredValue::new(w.clone());
    let fetch = move |apply: bool| {
        let targets = apply.then(|| ctx.targets.get_untracked());
        tiles.fetch(ctx, wallpaper.get_value(), targets);
    };

    let purity = w.purity.clone();
    let not_sfw = purity != "sfw";
    // Like the Noctalia panel: the picture is its own rounded box with the
    // rating as its border (green sfw, yellow sketchy, red nsfw; fixed
    // colors, so they read the same in any theme), the caption plain below.
    let ring = match purity.as_str() {
        "nsfw" => "ring-red-500",
        "sketchy" => "ring-yellow-400",
        _ => "ring-green-500",
    };

    view! {
        <div data-menu-key=key class="group flex flex-col gap-1.5 transition duration-300 ease-bounce hover:-translate-y-1">
            <div class=format!(
                "relative aspect-[16/10] w-full overflow-hidden rounded-2xl bg-muted shadow-md shadow-black/10 ring-2 {ring} transition group-hover:shadow-lg group-hover:ring-[3px]",
            )>
                <img src=w.thumb.clone() alt="" loading="lazy" class="size-full object-cover" />
                // Clicking the picture itself (not a button) opens the details.
                <div
                    class="absolute inset-0 flex cursor-pointer items-end justify-end gap-2 bg-gradient-to-t from-black/60 to-transparent p-2 opacity-0 transition group-focus-within:opacity-100 group-hover:opacity-100"
                    on:click=move |e| {
                        if e.target() == e.current_target() {
                            online.show_details(wallpaper.get_value());
                        }
                    }
                >
                    <Button
                        size=ButtonSize::Sm
                        variant=ButtonVariant::Ghost
                        class="mr-auto text-white hover:text-foreground"
                        on_click=move |_| online.show_details(wallpaper.get_value())
                    >
                        "Details"
                    </Button>
                    <Button size=ButtonSize::Sm disabled=working on_click=move |_| fetch(true)>
                        "Apply"
                    </Button>
                    <Show when=move || !saved.get()>
                        <Button
                            size=ButtonSize::Sm
                            variant=ButtonVariant::Secondary
                            disabled=working
                            on_click=move |_| fetch(false)
                        >
                            "Download"
                        </Button>
                    </Show>
                </div>
                <div class="absolute top-2 left-2 flex gap-1">
                    <Show when=move || saved.get()>
                        <Tip text="Already in your download folder">
                            <Badge variant=BadgeVariant::Secondary>
                                <Check />
                                "Downloaded"
                            </Badge>
                        </Tip>
                    </Show>
                    {not_sfw.then(|| view! { <Badge variant=BadgeVariant::Destructive>{purity.clone()}</Badge> })}
                </div>
                <Show when=move || working.get()>
                    <div class="absolute inset-0 flex items-center justify-center bg-background/60">
                        <Spinner />
                    </div>
                </Show>
            </div>
            <div class="flex justify-between px-1 text-xs">
                <span>{w.resolution()}</span>
                <span class="text-muted-foreground">{likes_label(&w)}</span>
            </div>
        </div>
    }
}
