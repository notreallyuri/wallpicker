//! Settings → Workspaces: a wallpaper per Hyprland workspace. The switches
//! are saved with the form; the list (each workspace and its wallpaper) is
//! the app's own and changes at once: as you apply wallpapers (with
//! "remember" on), and from here: a workspace's Choose turns the section
//! into a picker over the library (in place, not a dialog over Settings).

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::utils::types::Orientation;

use nocomponents::icons::search::Search as SearchIcon;
use wallpicker_core::library::LocalImage;

use crate::api::{self, WorkspaceRow};
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

#[component]
pub(crate) fn WorkspacesPane(enabled: RwSignal<bool>, remember: RwSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let rows = RwSignal::new(None::<Vec<WorkspaceRow>>);
    let load = move || {
        spawn_local(async move {
            match api::workspace_rows().await {
                Ok(r) => {
                    rows.try_set(Some(r));
                }
                Err(e) => ctx.error_with("Could not list the workspaces", &e),
            }
        });
    };
    load();
    // Workspaces open and close, and switches change what's shown: kept up
    // to date while this section is open.
    let alive = RwSignal::new(true);
    on_cleanup(move || alive.set(false));
    let _ = set_interval_with_handle(
        move || {
            if alive.get_untracked() {
                load();
            }
        },
        std::time::Duration::from_secs(3),
    )
    .map(|h| on_cleanup(move || h.clear()));
    // The workspace being chosen for (the picker shows instead of the list).
    let choosing = RwSignal::new(None::<String>);
    let assign = move |ws: String, path: Option<String>| {
        choosing.set(None);
        spawn_local(async move {
            match api::set_workspace_wallpaper(&ws, path.as_deref()).await {
                Ok(()) => load(),
                Err(e) => ctx.error_with("Could not change the workspace's wallpaper", &e),
            }
        });
    };

    view! {
        // While choosing, the picker has the section to itself.
        <div class="flex flex-col gap-6" class:hidden=move || choosing.with(Option::is_some)>
            <Field orientation=Orientation::Horizontal>
                <Switch checked=enabled />
                <FieldLabel>"A wallpaper per workspace"</FieldLabel>
            </Field>
            <p class="-mt-2 text-sm text-muted-foreground">
                "When a monitor switches to a workspace that has a wallpaper, it goes on that monitor. "
                "Workspaces without one leave the monitor as it is. Switching doesn't count as a pick: "
                "Previous still goes back to the one you chose before. Save to apply."
            </p>
            <Field orientation=Orientation::Horizontal>
                <Switch checked=remember />
                <FieldLabel>"Remember what I put on each workspace"</FieldLabel>
            </Field>
            <p class="-mt-2 text-sm text-muted-foreground">
                "A wallpaper you apply (or go back to) becomes the wallpaper of the workspace its monitor is "
                "showing. Random and the slideshow don't change them."
            </p>
        </div>
        <Show when=move || choosing.with(Option::is_some)>
            <Picker workspace=Signal::derive(move || choosing.get().unwrap_or_default()) assign=assign back=move || choosing.set(None) />
        </Show>
        <div class="flex flex-col gap-2" class:hidden=move || choosing.with(Option::is_some)>
            <span class="text-sm font-bold">"Workspaces"</span>
            {move || match rows.get() {
                None => view! { <Spinner class="size-4" /> }.into_any(),
                Some(r) if r.is_empty() => {
                    view! { <p class="text-sm text-muted-foreground">"No workspaces (is Hyprland running?)."</p> }.into_any()
                }
                Some(r) => {
                    r.into_iter()
                        .map(|row| view! { <WorkspaceItem row=row assign=assign choose=move |ws| choosing.set(Some(ws)) /> })
                        .collect_view()
                        .into_any()
                }
            }}
            <p class="text-xs text-muted-foreground">
                "Also from a terminal: " <code>"wallpicker workspaces"</code>
                ". wallpicker follows the workspaces while it runs (the tray is enough); without it, run "
                <code>"wallpicker workspaces run"</code> "."
            </p>
        </div>
    }
}

/// One workspace: its wallpaper's thumbnail, where it is, and buttons.
#[component]
fn WorkspaceItem(
    row: WorkspaceRow,
    assign: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    choose: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let thumb = RwSignal::new(None::<String>);
    if let Some(p) = row.wallpaper.clone() {
        spawn_local(async move {
            if let Ok(t) = api::thumbnail(&p).await {
                thumb.try_set(Some(api::asset_url(&t)));
            }
        });
    }
    let name = StoredValue::new(row.name.clone());
    let has_wallpaper = row.wallpaper.is_some();
    // What its monitor shows now: "Use it" when that's not its wallpaper.
    let shown = {
        let monitor = row.monitor.clone();
        let wallpaper = row.wallpaper.clone();
        Memo::new(move |_| {
            let m = monitor.as_ref()?;
            let p = ctx.state.with(|s| s.current.get(m).map(|p| p.to_string_lossy().into_owned()))?;
            (Some(&p) != wallpaper.as_ref()).then_some((m.clone(), p))
        })
    };
    let file = row.wallpaper.as_deref().and_then(|p| p.rsplit('/').next()).map(str::to_string);
    let where_ = match (&row.monitor, row.open) {
        (Some(m), _) => format!("on {m}"),
        (None, true) => "open".to_string(),
        (None, false) => "not open now".to_string(),
    };
    view! {
        <div class="flex items-center gap-3 rounded-2xl border-2 border-border/60 p-2">
            <div class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/15 font-black text-primary">
                {row.name.chars().take(3).collect::<String>()}
            </div>
            // The picture is a way in to choosing too.
            <button
                type="button"
                aria-label="Choose a wallpaper"
                on:click=move |_| choose(name.get_value())
                class=if has_wallpaper {
                    "group/pic relative h-12 w-20 shrink-0 overflow-hidden rounded-lg bg-muted transition duration-200 ease-bounce hover:-translate-y-0.5"
                } else {
                    // Nothing yet: an empty slot that says what it does.
                    "flex h-12 w-20 shrink-0 items-center justify-center rounded-lg border-2 border-dashed border-border text-xs font-bold text-muted-foreground transition hover:border-primary hover:text-primary"
                }
            >
                {if has_wallpaper {
                    view! {
                        {move || thumb.get().map(|t| view! { <img src=t alt="" class="size-full object-cover" /> })}
                        <span class="absolute inset-0 flex items-center justify-center bg-black/45 text-xs font-bold text-white opacity-0 transition group-hover/pic:opacity-100">
                            "Choose"
                        </span>
                    }
                        .into_any()
                } else {
                    "+ Choose".into_any()
                }}
            </button>
            <div class="flex min-w-0 flex-1 flex-col">
                <span class="truncate text-sm font-bold">
                    {file.unwrap_or_else(|| "No wallpaper".to_string())}
                </span>
                <span class="truncate text-xs text-muted-foreground">{format!("Workspace {} · {where_}", row.name)}</span>
            </div>
            {move || shown.get().map(|(m, p)| view! {
                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| assign(name.get_value(), Some(p.clone()))>
                    {format!("Use {m}'s")}
                </Button>
            })}
            {row.wallpaper.is_some().then(|| view! {
                <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| assign(name.get_value(), None)>
                    "Clear"
                </Button>
            })}
        </div>
    }
}

/// How many pictures the picker shows at first, and adds as you scroll.
const STEP: usize = 36;

/// The library to choose a workspace's wallpaper from: favorites first, then
/// the newest; searchable by name and tag.
#[component]
fn Picker(
    workspace: Signal<String>,
    assign: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    back: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let images = RwSignal::new(None::<Vec<LocalImage>>);
    let query = RwSignal::new(String::new());
    let favorites = RwSignal::new(false);
    let limit = RwSignal::new(STEP);
    spawn_local(async move {
        match api::scan_local().await {
            Ok(list) => {
                images.try_set(Some(list));
            }
            Err(e) => ctx.error_with("Scanning folders failed", &e),
        }
    });
    let shown = Memo::new(move |_| {
        let q = query.get().trim().to_lowercase();
        let only_favorites = favorites.get();
        images.with(|list| {
            let Some(list) = list else { return Vec::new() };
            ctx.catalog.with(|c| {
                let mut out: Vec<(bool, LocalImage)> = list
                    .iter()
                    .filter(|i| !only_favorites || c.is_favorite(&i.path))
                    .filter(|i| {
                        q.is_empty()
                            || i.name.to_lowercase().contains(&q)
                            || c.tags(&i.path).iter().any(|t| t.to_lowercase().contains(&q))
                    })
                    .map(|i| (c.is_favorite(&i.path), i.clone()))
                    .collect();
                out.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.modified.cmp(&a.1.modified)));
                out.into_iter().map(|(_, i)| i).collect()
            })
        })
    });
    // A new search starts from the top.
    Effect::new(move |_| {
        query.track();
        favorites.track();
        limit.set(STEP);
    });
    view! {
        <div class="flex flex-col gap-3">
            <div class="flex items-center gap-2">
                <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| back()>
                    "← Back"
                </Button>
                <span class="min-w-0 flex-1 truncate text-sm font-bold">
                    {move || format!("Choose for workspace {}", workspace.get())}
                </span>
            </div>
            <div class="flex items-center gap-2">
                <div class="relative min-w-0 flex-1">
                    <SearchIcon class="pointer-events-none absolute top-2 left-2.5 text-muted-foreground" />
                    <Input model=query class="pl-8" attr:placeholder="Name or tag" />
                </div>
                // Rebuilt when toggled: a Button's variant is fixed once drawn.
                {move || {
                    let on = favorites.get();
                    view! {
                        <Button
                            variant=if on { ButtonVariant::Default } else { ButtonVariant::Outline }
                            on_click=move |_| favorites.set(!on)
                        >
                            "Favorites"
                        </Button>
                    }
                }}
            </div>
            <div
                class="grid max-h-[22rem] grid-cols-3 gap-2 overflow-y-auto pr-1"
                on:scroll=move |e| {
                    use wasm_bindgen::JsCast;
                    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
                    // Near the bottom: the next batch.
                    if el.scroll_top() + el.client_height() + 200 >= el.scroll_height() {
                        let total = shown.with(Vec::len);
                        limit.update(|l| *l = (*l + STEP).min(total.max(STEP)));
                    }
                }
            >
                {move || match images.with(Option::is_some) {
                    false => view! { <Spinner class="size-4" /> }.into_any(),
                    true if shown.with(Vec::is_empty) => {
                        view! { <p class="col-span-3 py-6 text-center text-sm text-muted-foreground">"Nothing matches."</p> }.into_any()
                    }
                    true => {
                        shown
                            .get()
                            .into_iter()
                            .take(limit.get())
                            .map(|img| view! { <PickTile img=img on_pick=move |p| assign(workspace.get_untracked(), Some(p)) /> })
                            .collect_view()
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// One picture in the picker: its thumbnail and name; a click picks it.
#[component]
fn PickTile(img: LocalImage, on_pick: impl Fn(String) + Copy + Send + Sync + 'static) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let src = RwSignal::new(None::<String>);
    let path = img.path.to_string_lossy().into_owned();
    {
        let path = path.clone();
        spawn_local(async move {
            if let Ok(t) = api::thumbnail(&path).await {
                src.try_set(Some(api::asset_url(&t)));
            }
        });
    }
    let key = StoredValue::new(img.path.clone());
    let favorite = move || ctx.catalog.with(|c| key.with_value(|p| c.is_favorite(p)));
    let pick = StoredValue::new(path);
    view! {
        <button
            type="button"
            on:click=move |_| on_pick(pick.get_value())
            class="group/tile flex min-w-0 flex-col gap-1 rounded-xl p-1 text-left transition duration-200 ease-bounce hover:-translate-y-0.5 hover:bg-primary/10"
        >
            <span class="relative block aspect-video w-full overflow-hidden rounded-lg bg-muted">
                {move || src.get().map(|s| view! { <img src=s alt="" loading="lazy" class="size-full object-cover" /> })}
                <Show when=favorite>
                    <span class="absolute top-1 right-1 text-xs text-amber-400 drop-shadow">"★"</span>
                </Show>
            </span>
            <span class="truncate px-0.5 text-[11px] text-muted-foreground">{img.name.clone()}</span>
        </button>
    }
}
