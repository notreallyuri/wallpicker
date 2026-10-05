//! The duplicates dialog: exact copies and the same picture at other sizes,
//! a group at a time. Click the copy to keep (the best one is picked
//! already), then trash the others: their favorite and tags move to it, and
//! a monitor showing one of them gets it instead.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wallpicker_core::duplicates::{DuplicateGroup, DuplicateItem, Likeness};

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

/// "2.1 MB".
fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

fn file_name(item: &DuplicateItem) -> String {
    item.path.file_name().map_or_else(|| item.path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

#[component]
pub(super) fn DuplicatesDialog(open: RwSignal<bool>) -> impl IntoView {
    // None while looking.
    let groups = RwSignal::new(None::<Result<Vec<DuplicateGroup>, String>>);
    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        groups.set(None);
        spawn_local(async move {
            groups.try_set(Some(api::find_duplicates().await));
        });
    });
    let drop_group = Callback::new(move |first: String| {
        groups.update(|g| {
            if let Some(Ok(list)) = g {
                list.retain(|grp| grp.items[0].path.to_string_lossy() != first);
            }
        });
    });
    view! {
        <Dialog open=open>
            <DialogPortal class="sm:max-w-3xl">
                <DialogHeader>
                    <DialogTitle>"Duplicates"</DialogTitle>
                    <DialogDescription>
                        "Exact copies and the same picture at another size. The copy to keep is picked "
                        "already (the largest); click another to keep it instead."
                    </DialogDescription>
                </DialogHeader>
                <div class="flex max-h-[65vh] flex-col gap-4 overflow-y-auto pr-1">
                    {move || match groups.get() {
                        None => view! { <div class="flex items-center gap-2 text-sm text-muted-foreground"><Spinner />"Looking…"</div> }.into_any(),
                        Some(Err(e)) => view! { <p class="text-sm text-destructive">{format!("Could not look for duplicates: {e}")}</p> }.into_any(),
                        Some(Ok(list)) if list.is_empty() => view! { <p class="text-sm text-muted-foreground">"No duplicates."</p> }.into_any(),
                        Some(Ok(list)) => list
                            .into_iter()
                            .map(|g| view! { <GroupCard group=g on_done=drop_group /> })
                            .collect_view()
                            .into_any(),
                    }}
                </div>
                <DialogFooter>
                    <Button variant=ButtonVariant::Ghost on_click=move |_| open.set(false)>"Close"</Button>
                </DialogFooter>
            </DialogPortal>
        </Dialog>
    }
}

#[component]
fn GroupCard(group: DuplicateGroup, on_done: Callback<String>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let keep = RwSignal::new(0usize);
    let working = RwSignal::new(false);
    let items = StoredValue::new(group.items.clone());
    let first = group.items[0].path.to_string_lossy().into_owned();
    let label = match group.likeness {
        Likeness::Exact => "Exact copies",
        Likeness::Similar => "Same picture",
    };
    let resolve = move |_| {
        let all = items.get_value();
        let k = keep.get_untracked();
        let kept = all[k].path.to_string_lossy().into_owned();
        let others: Vec<String> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != k)
            .map(|(_, it)| it.path.to_string_lossy().into_owned())
            .collect();
        let first = first.clone();
        working.set(true);
        spawn_local(async move {
            match api::resolve_duplicates(&kept, &others).await {
                Ok(r) if r.failed.is_empty() => {
                    ctx.info(format!("Moved {} to the trash", r.trashed.len()));
                    on_done.run(first);
                }
                Ok(r) => {
                    let detail = r.failed.iter().map(|(p, why)| format!("{p}: {why}")).collect::<Vec<_>>().join("\n");
                    ctx.error_with("Some copies could not be moved to the trash", &detail);
                }
                Err(e) => ctx.error_with("Could not clear the duplicates", &e),
            }
            working.try_set(false);
            ctx.library_rev.update(|r| *r += 1);
            ctx.refresh_catalog().await;
            ctx.refresh_state().await;
        });
    };
    let first_for_skip = group.items[0].path.to_string_lossy().into_owned();
    let n_others = group.items.len() - 1;
    view! {
        <div class="flex flex-col gap-3 rounded-xl border p-3">
            <div class="flex items-center justify-between text-sm">
                <span class="font-medium">{label}</span>
                <span class="text-muted-foreground">{format!("{} files", group.items.len())}</span>
            </div>
            <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                {group
                    .items
                    .into_iter()
                    .enumerate()
                    .map(|(i, item)| view! { <CopyTile item=item index=i keep=keep /> })
                    .collect_view()}
            </div>
            <div class="flex items-center justify-end gap-2">
                <Button variant=ButtonVariant::Ghost on_click=move |_| on_done.run(first_for_skip.clone())>
                    "Skip"
                </Button>
                <Button variant=ButtonVariant::Destructive disabled=working on_click=resolve>
                    {format!("Keep it, trash the other{}", if n_others == 1 { "" } else { "s" })}
                </Button>
            </div>
        </div>
    }
}

/// A copy: its picture, size and file; click to keep it.
#[component]
fn CopyTile(item: DuplicateItem, index: usize, keep: RwSignal<usize>) -> impl IntoView {
    let src = RwSignal::new(None::<String>);
    let path = item.path.to_string_lossy().into_owned();
    spawn_local(async move {
        if let Ok(thumb) = api::thumbnail(&path).await {
            src.try_set(Some(api::asset_url(&thumb)));
        }
    });
    let kept = move || keep.get() == index;
    let name = file_name(&item);
    view! {
        <button
            type="button"
            title=item.path.display().to_string()
            on:click=move |_| keep.set(index)
            class=move || {
                format!(
                    "flex flex-col gap-1 rounded-lg p-1 text-left text-xs transition {}",
                    if kept() { "ring-2 ring-primary" } else { "opacity-60 ring-1 ring-foreground/10 hover:opacity-100" },
                )
            }
        >
            <div class="relative aspect-[16/10] w-full overflow-hidden rounded-md bg-muted">
                {move || src.get().map(|s| view! { <img src=s alt="" class="size-full object-cover" /> })}
                <span class=move || {
                    format!(
                        "absolute top-1 left-1 rounded px-1.5 py-0.5 text-[10px] font-medium {}",
                        if kept() { "bg-primary text-primary-foreground" } else { "bg-destructive/80 text-white" },
                    )
                }>{move || if kept() { "Keep" } else { "Trash" }}</span>
            </div>
            <span class="truncate">{name}</span>
            <span class="text-muted-foreground">{format!("{}×{} · {}", item.width, item.height, megabytes(item.size))}</span>
        </button>
    }
}
