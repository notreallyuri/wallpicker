//! The tag filter above the grid, and the dialog that edits one wallpaper's
//! tags.

use std::path::Path;

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::icons::x::X;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use wallpicker_core::catalog::{TagMatch, normalize_tag};

use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

const MATCH_ALL: &str = "all";
const MATCH_ANY: &str = "any";

/// Tag filter state, shared by the filter row and the grid.
#[derive(Clone, Copy)]
pub(super) struct TagFilter {
    /// Selected tags. Bound to a multi-select ToggleGroup.
    pub(super) selected: RwSignal<Vec<String>>,
    /// [`MATCH_ALL`] or [`MATCH_ANY`]. Bound to a single-select ToggleGroup.
    mode: RwSignal<Vec<String>>,
}

impl TagFilter {
    pub(super) fn new() -> TagFilter {
        let mode = RwSignal::new(vec![MATCH_ALL.to_string()]);
        // Clicking the active All/Any again would clear it; keep one chosen.
        Effect::new(move |_| {
            if mode.with(Vec::is_empty) {
                mode.set(vec![MATCH_ALL.to_string()]);
            }
        });
        TagFilter {
            selected: RwSignal::new(Vec::new()),
            mode,
        }
    }

    pub(super) fn mode(self) -> TagMatch {
        if self.mode.with(|m| m.iter().any(|m| m == MATCH_ANY)) {
            TagMatch::Any
        } else {
            TagMatch::All
        }
    }
}

/// One toggle per tag in use, with how many wallpapers have it, on one line
/// (selected tags first) with Expand/Collapse when they don't fit. All/Any
/// appears once two or more tags are selected.
#[component]
pub(super) fn TagFilterRow(
    filter: TagFilter,
    /// (tag, count) for the wallpapers on disk.
    counts: Memo<Vec<(String, usize)>>,
) -> impl IntoView {
    let expanded = RwSignal::new(crate::prefs::get("local.tags_expanded").as_deref() == Some("1"));
    Effect::new(move |_| crate::prefs::set("local.tags_expanded", if expanded.get() { "1" } else { "0" }));
    // Selected first, so a collapsed row never hides one that's on.
    let ordered = move || {
        let selected = filter.selected.get();
        let mut tags = counts.get();
        tags.sort_by_key(|(t, _)| !selected.contains(t));
        tags
    };
    // Whether the chips run past one line: measured after they're drawn.
    let list = NodeRef::<leptos::html::Div>::new();
    let overflowing = RwSignal::new(false);
    Effect::new(move |_| {
        counts.track();
        filter.selected.track();
        expanded.track();
        request_animation_frame(move || {
            if let Some(el) = list.get_untracked() {
                overflowing.try_set(el.scroll_height() > el.client_height() + 1);
            }
        });
    });
    view! {
        <Show when=move || counts.with(|c| !c.is_empty())>
            <div class="flex items-start gap-2">
                <span class="text-sm leading-7 text-muted-foreground">"Tags"</span>
                <div
                    node_ref=list
                    class=move || {
                        if expanded.get() { "min-w-0 flex-1" } else { "max-h-7 min-w-0 flex-1 overflow-hidden" }
                    }
                >
                    <ToggleGroup
                        kind=ToggleGroupType::Multiple
                        variant=ToggleVariant::Outline
                        size=ToggleSize::Sm
                        value=filter.selected
                        class="flex-wrap gap-1.5"
                    >
                        <For
                            each=ordered
                            key=|(tag, n)| (tag.clone(), *n)
                            children=|(tag, n): (String, usize)| {
                                view! {
                                    <ToggleGroupItem value=tag.clone() class="rounded-full! border-primary/25! bg-primary/5 px-3 hover:bg-primary/15">
                                        {tag}
                                        <span class="text-muted-foreground">{n}</span>
                                    </ToggleGroupItem>
                                }
                            }
                        />
                    </ToggleGroup>
                </div>
                <Show when=move || filter.selected.with(|s| s.len() > 1)>
                    <ToggleGroup
                        kind=ToggleGroupType::Single
                        variant=ToggleVariant::Outline
                        size=ToggleSize::Sm
                        value=filter.mode
                    >
                        <ToggleGroupItem value=MATCH_ALL attr:title="Wallpapers with every selected tag">
                            "All"
                        </ToggleGroupItem>
                        <ToggleGroupItem value=MATCH_ANY attr:title="Wallpapers with any selected tag">
                            "Any"
                        </ToggleGroupItem>
                    </ToggleGroup>
                </Show>
                <Show when=move || filter.selected.with(|s| !s.is_empty())>
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Sm
                        class="h-7"
                        on_click=move |_| filter.selected.set(Vec::new())
                    >
                        <X />
                        "Clear"
                    </Button>
                </Show>
                <Show when=move || overflowing.get() || expanded.get()>
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Sm
                        class="h-7"
                        on_click=move |_| expanded.update(|e| *e = !*e)
                    >
                        {move || if expanded.get() { "Collapse" } else { "Expand" }}
                    </Button>
                </Show>
            </div>
        </Show>
    }
}

/// Edits the tags of the wallpaper in `editing`.
#[component]
pub(super) fn TagsDialog(editing: RwSignal<Option<String>>, open: RwSignal<bool>) -> impl IntoView {
    view! {
        <Dialog open=open>
            <DialogPortal class="sm:max-w-md">
                {move || editing.get().map(|path| view! { <TagsBody path=path /> })}
            </DialogPortal>
        </Dialog>
    }
}

#[component]
fn TagsBody(path: String) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let input = RwSignal::new(String::new());
    let key = StoredValue::new(path.clone());
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());

    let current = move || ctx.catalog.with(|c| key.with_value(|p| c.tags(Path::new(p)).to_vec()));
    // Tags used elsewhere that this wallpaper doesn't have, matching what's typed.
    let suggestions = move || {
        let typed = input.get().trim().to_lowercase();
        ctx.catalog.with(|c| {
            let mine = key.with_value(|p| c.entry(Path::new(p)).cloned().unwrap_or_default());
            let mut all: Vec<String> = Vec::new();
            for tag in c.items.values().flat_map(|e| e.tags.iter()) {
                if !mine.has_tag(tag)
                    && tag.to_lowercase().contains(&typed)
                    && !all.iter().any(|t| t.to_lowercase() == tag.to_lowercase())
                {
                    all.push(tag.clone());
                }
            }
            all.sort_by_key(|t| t.to_lowercase());
            all.truncate(12);
            all
        })
    };
    let add = move |tag: String| {
        if normalize_tag(&tag).is_some() {
            input.set(String::new());
            spawn_local(ctx.add_tag(key.get_value(), tag));
        }
    };
    let valid = Signal::derive(move || input.with(|t| normalize_tag(t).is_some()));

    view! {
        <DialogHeader>
            <DialogTitle>"Tags"</DialogTitle>
            <DialogDescription class="truncate">{name}</DialogDescription>
        </DialogHeader>
        <div class="flex min-h-7 flex-wrap gap-1.5">
            <Show
                when=move || !current().is_empty()
                fallback=|| view! { <span class="text-muted-foreground">"No tags yet."</span> }
            >
                <For
                    each=current
                    key=|t| t.clone()
                    children=move |tag: String| {
                        let remove = tag.clone();
                        view! {
                            <span class="inline-flex items-center gap-1 rounded-full bg-secondary py-0.5 pr-1 pl-2.5 text-xs text-secondary-foreground">
                                {tag.clone()}
                                <button
                                    type="button"
                                    class="rounded-full p-0.5 hover:bg-background/60"
                                    aria-label=format!("Remove {tag}")
                                    on:click=move |_| {
                                        spawn_local(ctx.remove_tag(key.get_value(), remove.clone()))
                                    }
                                >
                                    <X class="size-3" />
                                </button>
                            </span>
                        }
                    }
                />
            </Show>
        </div>
        <form
            class="flex gap-2"
            on:submit=move |e| {
                e.prevent_default();
                add(input.get_untracked());
            }
        >
            <Input model=input attr:placeholder="Add a tag" attr:autofocus=true />
            <Button r#type=ButtonType::Submit disabled=Signal::derive(move || !valid.get())>
                "Add"
            </Button>
        </form>
        <Show when=move || !suggestions().is_empty()>
            <div class="flex flex-wrap gap-1.5">
                <For
                    each=suggestions
                    key=|t| t.clone()
                    children=move |tag: String| {
                        let t = tag.clone();
                        view! {
                            <Button
                                size=ButtonSize::Xs
                                variant=ButtonVariant::Outline
                                on_click=move |_| add(t.clone())
                            >
                                {format!("+ {tag}")}
                            </Button>
                        }
                    }
                />
            </div>
        </Show>
    }
}
