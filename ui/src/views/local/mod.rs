//! The Local tab: wallpapers found in the configured folders, with favorites
//! pinned first, a tag filter, a sort, and a Flat or Folders layout.

mod duplicates;
mod colors;
pub(crate) mod folders;
mod tags;
mod tile;
mod trash;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::icons::search::Search as SearchIcon;
use wallpicker_core::library::{self, LocalImage, Place, Subfolder};
use wallpicker_core::settings::WallpaperBackend;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::tile_menu::{MenuTarget, TileMenu};
use crate::views::{Choice, fit_from};
use crate::components::tip::Tip;

use duplicates::DuplicatesDialog;
use folders::{Breadcrumb, FolderCard, Lucide};

use tags::{TagFilter, TagFilterRow, TagsDialog};
use tile::LocalTile;
use trash::{TrashConfirm, TrashDialog};

/// Local tiles rendered per "Show more" step.
const PAGE: usize = 60;

const SORTS: &[(&str, &str)] = &[
    ("newest", "Newest"),
    ("oldest", "Oldest"),
    ("name", "Name"),
    ("random", "Random order"),
];

/// A scanned image and where it sits in the wallpaper folders.
#[derive(Clone, PartialEq)]
struct Placed {
    img: LocalImage,
    place: Place,
}

/// What the grid shows: subfolders (Folders layout, no filter on), with
/// their covers, then the images.
#[derive(Clone, PartialEq, Default)]
struct Shown {
    folders: Vec<(Subfolder, String)>,
    images: Vec<LocalImage>,
}

/// A wallpaper's own fit: "" is automatic (the monitor's).
const FITS: &[(&str, &str)] = &[
    ("", "Automatic"),
    ("cover", "Cover"),
    ("contain", "Contain"),
    ("tile", "Tile"),
    ("fill", "Fill"),
];



/// A remembered choice among `allowed`, or the first.
fn remembered(key: &str, allowed: &[&str]) -> String {
    crate::prefs::get(key)
        .filter(|v| allowed.contains(&v.as_str()))
        .unwrap_or_else(|| allowed[0].to_string())
}

/// Which wallpaper the tags dialog is editing.
#[derive(Clone, Copy)]
struct TagEditor {
    editing: RwSignal<Option<String>>,
    open: RwSignal<bool>,
}

#[component]
pub(crate) fn LocalView() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let images = RwSignal::new(Vec::<LocalImage>::new());
    let loading = RwSignal::new(true);
    let filter = RwSignal::new(String::new());
    let tag_filter = TagFilter::new();
    let limit = RwSignal::new(PAGE);
    let favorites_only = RwSignal::new(false);
    let duplicates_open = RwSignal::new(false);
    // The color filter: a hue, "dark", "light" or "muted"; "" is none.
    let color = RwSignal::new(String::new());
    // Every image's colors, read when a color is first picked (and again
    // after a rescan while one is).
    let profiles = RwSignal::new(None::<std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>>);
    let profiles_loading = RwSignal::new(false);
    Effect::new(move |seen: Option<u32>| {
        let rev = ctx.library_rev.get();
        let wanted = color.with(|c| !c.is_empty());
        if !wanted {
            return seen.unwrap_or(u32::MAX);
        }
        let stale = profiles.with_untracked(Option::is_none) || seen.is_some_and(|s| s != rev);
        if stale && !profiles_loading.get_untracked() {
            profiles_loading.set(true);
            spawn_local(async move {
                match api::image_colors().await {
                    Ok(p) => {
                        profiles.try_set(Some(p));
                    }
                    Err(e) => ctx.error_with("Could not read the wallpapers' colors", &e),
                }
                profiles_loading.try_set(false);
            });
        }
        rev
    });
    let sort = RwSignal::new(remembered("local.sort", &["newest", "oldest", "name", "random"]));
    let layout = RwSignal::new(remembered("local.layout", &["flat", "folders"]));
    Effect::new(move |_| crate::prefs::set("local.sort", &sort.get()));
    Effect::new(move |_| crate::prefs::set("local.layout", &layout.get()));
    // The "random" order: a number per path, drawn again each time Random
    // order is picked.
    let shuffle = RwSignal::new(HashMap::<PathBuf, u64>::new());
    Effect::new(move |prev: Option<String>| {
        let s = sort.get();
        if s == "random" && prev.as_deref() != Some("random") {
            let fresh = images.with_untracked(|all| {
                all.iter()
                    .map(|i| (i.path.clone(), (js_sys::Math::random() * 1e12) as u64))
                    .collect()
            });
            shuffle.set(fresh);
        }
        s
    });
    let editor = TagEditor {
        editing: RwSignal::new(None),
        open: RwSignal::new(false),
    };
    provide_context(editor);
    // `wallpicker --open duplicates` / `--open tags IMAGE`, on this tab
    // (switched here: the shell's effect may run after this one).
    let tabs = nocomponents::primitives::tabs::use_tabs();
    Effect::new(move |_| {
        let Some(place) = ctx.open_place.get() else { return };
        if matches!(place.view.as_str(), "duplicates" | "tags") {
            tabs.current_tab.set("local".to_string());
        }
        match place.view.as_str() {
            "duplicates" => duplicates_open.set(true),
            "tags" => {
                editor.editing.set(place.path.clone());
                editor.open.set(true);
            }
            _ => return,
        }
        ctx.open_place.set(None);
    });
    let trash = TrashConfirm::new();
    provide_context(trash);

    Effect::new(move |_| {
        ctx.library_rev.track();
        loading.set(true);
        spawn_local(async move {
            match api::scan_local().await {
                Ok(list) => {
                    images.set(list);
                    limit.set(PAGE);
                }
                Err(e) => ctx.error_with("Scanning folders failed", &e),
            }
            loading.set(false);
        });
    });

    // The wallpaper folders, and where the Folders layout is.
    let roots = Memo::new(move |_| {
        ctx.settings.with(|s| s.as_ref().map(|s| s.folders.clone()).unwrap_or_default())
    });
    let root_names = Memo::new(move |_| {
        roots.with(|r| {
            r.iter()
                .map(|p| p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()))
                .collect::<Vec<_>>()
        })
    });
    let place = RwSignal::new(library::top_place(1));
    let placed = Memo::new(move |_| {
        roots.with(|r| {
            images.with(|all| {
                all.iter()
                    .map(|i| {
                        let (root, folder) = library::folder_in(&i.path, r).unwrap_or((0, String::new()));
                        Placed { img: i.clone(), place: Place { root, folder } }
                    })
                    .collect::<Vec<_>>()
            })
        })
    });
    // Back to the top when the folders change, or the place is gone.
    Effect::new(move |_| {
        let n = roots.with(Vec::len);
        let here = place.get_untracked();
        let fine = match &here {
            None => n > 1,
            Some(p) => {
                p.root < n
                    && (p.folder.is_empty() || placed.with(|all| all.iter().any(|x| p.contains(&x.place))))
            }
        };
        if !fine {
            place.set(library::top_place(n));
        }
    });

    // A new filter, sort, layout or folder starts from the first page again.
    Effect::new(move |_| {
        filter.track();
        tag_filter.selected.track();
        favorites_only.track();
        color.track();
        sort.track();
        layout.track();
        place.track();
        limit.set(PAGE);
    });

    // Tags of the wallpapers on disk, for the filter row.
    let counts = Memo::new(move |_| {
        ctx.catalog.with(|c| {
            images.with(|all| c.tag_counts(all.iter().map(|i| i.path.as_path())))
        })
    });
    // A selected tag that no longer exists (its last use was removed) would
    // filter everything out while no longer being shown, so drop it.
    Effect::new(move |_| {
        let live = counts.get();
        let stale = tag_filter
            .selected
            .with_untracked(|s| s.iter().any(|t| !live.iter().any(|(l, _)| l == t)));
        if stale {
            tag_filter
                .selected
                .update(|s| s.retain(|t| live.iter().any(|(l, _)| l == t)));
        }
    });

    let filtering = Signal::derive(move || {
        favorites_only.get()
            || !filter.with(String::is_empty)
            || tag_filter.selected.with(|t| !t.is_empty())
            || color.with(|c| !c.is_empty())
    });

    // Favorites first, then the chosen order.
    let sorted = Memo::new(move |_| {
        let mode = sort.get();
        let mut list = placed.get();
        ctx.catalog.with(|c| {
            shuffle.with(|sh| {
                list.sort_by(|a, b| {
                    let fav = c.is_favorite(&b.img.path).cmp(&c.is_favorite(&a.img.path));
                    let order = match mode.as_str() {
                        "oldest" => a.img.modified.cmp(&b.img.modified),
                        "name" => a.img.name.to_lowercase().cmp(&b.img.name.to_lowercase()),
                        "random" => sh.get(&a.img.path).cmp(&sh.get(&b.img.path)),
                        _ => b.img.modified.cmp(&a.img.modified),
                    };
                    fav.then(order).then_with(|| a.img.path.cmp(&b.img.path))
                });
            })
        });
        list
    });

    let shown = Memo::new(move |_| {
        let text = filter.get().to_lowercase();
        let selected = tag_filter.selected.get();
        let mode = tag_filter.mode();
        let favorites = favorites_only.get();
        let folders_layout = layout.get() == "folders";
        let here = place.get();
        sorted.with(|all| {
            if folders_layout && !filtering.get() {
                let places: Vec<Place> = all.iter().map(|p| p.place.clone()).collect();
                let (subs, direct) = root_names.with(|names| library::browse(&places, here.as_ref(), names));
                return Shown {
                    folders: subs
                        .into_iter()
                        .map(|f| {
                            let cover = all[f.first].img.path.to_string_lossy().into_owned();
                            (f, cover)
                        })
                        .collect(),
                    images: direct.into_iter().map(|i| all[i].img.clone()).collect(),
                };
            }
            let hue = color.get();
            let mut images: Vec<LocalImage> = ctx.catalog.with(|c| {
                profiles.with(|prof| {
                    all.iter()
                        .filter(|p| !folders_layout || library::place_inside(&p.place, here.as_ref()))
                        .filter(|p| !favorites || c.is_favorite(&p.img.path))
                        .filter(|p| {
                            // The text filter matches file names and tags.
                            text.is_empty()
                                || p.img.name.to_lowercase().contains(&text)
                                || c.tags(&p.img.path).iter().any(|t| t.to_lowercase().contains(&text))
                        })
                        .filter(|p| c.matches(&p.img.path, &selected, mode))
                        // Until the colors are read, nothing matches a color.
                        .filter(|p| {
                            hue.is_empty()
                                || prof.as_ref().and_then(|m| m.get(&p.img.path)).is_some_and(|cp| wallpicker_core::colors::matches(cp, &hue))
                        })
                        .map(|p| p.img.clone())
                        .collect()
                })
            });
            // With a color: the most of it first.
            if !hue.is_empty() {
                profiles.with(|prof| {
                    if let Some(m) = prof {
                        let score = |i: &LocalImage| m.get(&i.path).map_or(0.0, |cp| wallpicker_core::colors::score(cp, &hue));
                        images.sort_by(|a, b| score(b).total_cmp(&score(a)));
                    }
                });
            }
            Shown { folders: Vec::new(), images }
        })
    });

    // A random one of those shown, not one already on the target monitors
    // when there's any other.
    let random = move |_| {
        let on_now: Vec<PathBuf> = ctx.state.with_untracked(|s| {
            ctx.targets
                .get_untracked()
                .iter()
                .filter_map(|m| s.current.get(m).cloned())
                .collect()
        });
        let pick = shown.with_untracked(|s| {
            let fresh: Vec<&LocalImage> = s.images.iter().filter(|i| !on_now.contains(&i.path)).collect();
            let pool: Vec<&LocalImage> = if fresh.is_empty() { s.images.iter().collect() } else { fresh };
            (!pool.is_empty()).then(|| {
                let i = ((js_sys::Math::random() * pool.len() as f64) as usize).min(pool.len() - 1);
                pool[i].path.to_string_lossy().into_owned()
            })
        });
        if let Some(path) = pick {
            spawn_local(ctx.apply(path));
        }
    };
    let open_folder = Callback::new(move |to: Option<Place>| place.set(to));

    let visible = move || {
        let n = limit.get();
        shown.with(|v| v.images.iter().take(n).cloned().collect::<Vec<_>>())
    };

    let folders = move || {
        ctx.settings.with(|s| {
            s.as_ref()
                .map(|s| {
                    s.folders
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default()
        })
    };

    view! {
        // Floats over the grid's scroll area (see heads.rs).
        <div data-tab-head class="absolute top-0 left-0 right-[var(--sb-w,0px)] z-20 border-b border-border/60 bg-background/70 px-5 pt-3 pb-2.5 backdrop-blur-xl flex flex-col gap-2">
        // Wraps only in a narrow window, rather than pushing buttons off the edge.
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
            // A fixed width: the row's other parts (the count) change, the box doesn't.
            <div class="relative w-64 shrink-0 xl:w-80">
                <SearchIcon class="pointer-events-none absolute top-2 left-2.5 text-muted-foreground" />
                <Input
                    model=filter
                    class="pl-8"
                    attr:placeholder="Filter by name or tag"
                    attr:data-shortcut="search"
                />
            </div>
            // Rebuilt when toggled: a Button's variant is fixed once drawn.
            {move || {
                let on = favorites_only.get();
                view! {
                    <Button
                        variant=if on { ButtonVariant::Default } else { ButtonVariant::Outline }
                        attr:aria-pressed=on.to_string()
                        on_click=move |_| favorites_only.set(!on)
                    >
                        <tile::StarIcon filled=on />
                        "Favorites"
                    </Button>
                }
            }}
            <Choice items=SORTS.to_vec() model=sort class="w-36" />
            <colors::ColorPicker color=color loading=profiles_loading />
            <Tip text="Show everything at once, or browse folder by folder">
            <Button
                variant=ButtonVariant::Outline
                on_click=move |_| layout.update(|l| *l = if l == "folders" { "flat".into() } else { "folders".into() })
            >
                {move || {
                    if layout.get() == "folders" {
                        view! {
                            <Lucide class="size-4">
                                <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                            </Lucide>
                            "Folders"
                        }
                            .into_any()
                    } else {
                        view! {
                            <Lucide class="size-4">
                                <rect width="7" height="7" x="3" y="3" rx="1" />
                                <rect width="7" height="7" x="14" y="3" rx="1" />
                                <rect width="7" height="7" x="14" y="14" rx="1" />
                                <rect width="7" height="7" x="3" y="14" rx="1" />
                            </Lucide>
                            "Flat"
                        }
                            .into_any()
                    }
                }}
            </Button>
            </Tip>
            <Tip text="Apply a random one of those shown (Ctrl+R)">
            <Button
                variant=ButtonVariant::Outline
                attr:data-shortcut="random"
                disabled=Signal::derive(move || ctx.applying.get().is_some() || shown.with(|s| s.images.is_empty()))
                on_click=random
            >
                <Lucide class="size-4">
                    <path d="m18 14 4 4-4 4" />
                    <path d="m18 2 4 4-4 4" />
                    <path d="M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-7.6a4 4 0 0 1 3.3-1.7H22" />
                    <path d="M2 6h1.972a4 4 0 0 1 3.6 2.2" />
                    <path d="M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45" />
                </Lucide>
                "Random"
            </Button>
            </Tip>
            <span class="min-w-24 shrink-0 text-sm whitespace-nowrap text-muted-foreground tabular-nums">
                {move || {
                    if loading.get() {
                        "Scanning…".to_string()
                    } else {
                        match shown.with(|v| v.images.len()) {
                            1 => "1 image".to_string(),
                            n => format!("{n} images"),
                        }
                    }
                }}
            </span>
            // Icons, with their names in tooltips: the row stays one line.
            <Tip text="Duplicates: copies and the same picture at other sizes" class="ml-auto">
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::Icon
                attr:aria-label="Duplicates"
                on_click=move |_| duplicates_open.set(true)
            >
                <Lucide class="size-4">
                    <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
                    <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
                </Lucide>
            </Button>
            </Tip>
            <Tip text="Rescan the wallpaper folders">
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::Icon
                attr:aria-label="Rescan"
                disabled=loading
                on_click=move |_| ctx.library_rev.update(|r| *r += 1)
            >
                <Lucide class="size-4">
                    <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
                    <path d="M21 3v5h-5" />
                    <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
                    <path d="M8 16H3v5" />
                </Lucide>
            </Button>
            </Tip>
        </div>

        <TagFilterRow filter=tag_filter counts=counts />

        <Show when=move || layout.get() == "folders">
            <Breadcrumb place=place root_names=root_names filtering=filtering />
        </Show>
        </div>

        <div data-scroll-root="local" class="relative min-h-0 flex-1 scroll-pt-[var(--head-h,0px)] overflow-y-auto px-5 pb-5 pt-[calc(var(--head-h,0px)+1rem)]">
        <Show when=move || !loading.get() && images.with(|v| v.is_empty())>
            <Empty>
                <EmptyHeader>
                    <EmptyTitle>"No images found"</EmptyTitle>
                    <EmptyDescription>
                        "Looking in: " {folders} ". Add folders in Settings, or download "
                        "something from the Wallhaven tab."
                    </EmptyDescription>
                </EmptyHeader>
            </Empty>
        </Show>

        <TileMenu
            on_apply=Callback::new(move |(path, monitors)| {
                spawn_local(ctx.apply_to(path, monitors))
            })
            extra=std::sync::Arc::new(|| view! { <LocalMenuItems /> }.into_any())
        >
            <For
                each=move || shown.with(|s| s.folders.clone())
                key=|(f, _)| f.place.clone()
                children=move |(f, cover): (Subfolder, String)| {
                    view! { <FolderCard sub=f cover=cover on_open=open_folder /> }
                }
            />
            <For
                each=visible
                key=|i| i.path.clone()
                children=|img: LocalImage| view! { <LocalTile img=img /> }
            />
        </TileMenu>

        <Show when=move || {
            !loading.get() && !images.with(Vec::is_empty) && shown.with(|s| s.folders.is_empty() && s.images.is_empty())
        }>
            <p class="py-10 text-center text-sm text-muted-foreground">"No wallpapers match."</p>
        </Show>

        <Show when=move || { shown.with(|v| v.images.len()) > limit.get() }>
            <div class="mt-6 flex justify-center">
                <Button
                    variant=ButtonVariant::Outline
                    on_click=move |_| limit.update(|l| *l += PAGE)
                >
                    "Show more"
                </Button>
            </div>
        </Show>

        </div>
        <TagsDialog editing=editor.editing open=editor.open />
        <DuplicatesDialog open=duplicates_open />
        <TrashDialog confirm=trash />
    }
}

/// The Local-only entries of the grid's right-click menu.
#[component]
fn LocalMenuItems() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let editor = expect_context::<TagEditor>();
    let trash = expect_context::<TrashConfirm>();
    let MenuTarget(target) = expect_context::<MenuTarget>();
    let favorite = move || {
        target.with(|t| {
            t.as_deref()
                .is_some_and(|p| ctx.catalog.with(|c| c.is_favorite(Path::new(p))))
        })
    };

    view! {
        <ContextMenuSeparator />
        <ContextMenuItem on_click=move |_| {
            if let Some(path) = target.get_untracked() {
                spawn_local(ctx.set_favorite(path, !favorite()));
            }
        }>
            {move || if favorite() { "Remove from favorites" } else { "Add to favorites" }}
        </ContextMenuItem>
        <ContextMenuItem on_click=move |_| {
            if let Some(path) = target.get_untracked() {
                editor.editing.set(Some(path));
                editor.open.set(true);
            }
        }>"Edit tags…"</ContextMenuItem>
        <ContextMenuSub>
            <ContextMenuSubTrigger>"Fit"</ContextMenuSubTrigger>
            <ContextMenuSubContent>
                {FITS
                    .iter()
                    .map(|&(value, label)| {
                        let fit = fit_from(value);
                        let checked = Signal::derive(move || {
                            target.with(|t| {
                                t.as_deref().is_some_and(|p| ctx.catalog.with(|c| c.fit(Path::new(p))) == fit)
                            })
                        });
                        view! {
                            <ContextMenuCheckboxItem
                                checked=checked
                                on_change=Callback::new(move |_: bool| {
                                    if let Some(path) = target.get_untracked() {
                                        spawn_local(ctx.set_fit(path, fit));
                                    }
                                })
                            >
                                // awww can't tile: it centers.
                                {move || {
                                    let awww = ctx.settings.with(|s| {
                                        s.as_ref().is_some_and(|s| s.wallpaper_backend == WallpaperBackend::Awww)
                                    });
                                    if awww && value == "tile" { "Centered" } else { label }
                                }}
                            </ContextMenuCheckboxItem>
                        }
                    })
                    .collect_view()}
                <ContextMenuSeparator />
                <p class="max-w-56 px-2 py-1 text-xs text-muted-foreground">
                    "Automatic uses the monitor's fit (Settings). Not with Noctalia (it has its own)."
                </p>
            </ContextMenuSubContent>
        </ContextMenuSub>
        <ContextMenuSeparator />
        <ContextMenuItem on_click=move |_| {
            if let Some(path) = target.get_untracked() {
                spawn_local(async move {
                    if let Err(e) = api::show_in_folder(&path).await {
                        ctx.error_with("Could not show it in its folder", &e);
                    }
                });
            }
        }>"Show in folder"</ContextMenuItem>
        <ContextMenuItem on_click=move |_| {
            if let Some(path) = target.get_untracked() {
                spawn_local(async move {
                    match api::copy_text(&path).await {
                        Ok(()) => ctx.info("Path copied"),
                        Err(e) => ctx.error_with("Could not copy the path", &e),
                    }
                });
            }
        }>"Copy path"</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem
            class="text-destructive focus:text-destructive"
            on_click=move |_| {
                if let Some(path) = target.get_untracked() {
                    trash.ask(path);
                }
            }
        >
            "Move to trash…"
        </ContextMenuItem>
    }
}
