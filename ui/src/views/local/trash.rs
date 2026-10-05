//! Moving a local wallpaper to the trash, after a confirmation.

use std::path::Path;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

/// Which wallpaper the confirmation is about. Opened from the right-click
/// menu, or the Delete key on a tile.
#[derive(Clone, Copy)]
pub(super) struct TrashConfirm {
    target: RwSignal<Option<String>>,
    open: RwSignal<bool>,
}

impl TrashConfirm {
    pub(super) fn new() -> TrashConfirm {
        TrashConfirm {
            target: RwSignal::new(None),
            open: RwSignal::new(false),
        }
    }

    pub(super) fn ask(self, path: String) {
        self.target.set(Some(path));
        self.open.set(true);
    }
}

#[component]
pub(super) fn TrashDialog(confirm: TrashConfirm) -> impl IntoView {
    view! {
        <Dialog open=confirm.open>
            <DialogPortal class="sm:max-w-md">
                {move || confirm.target.get().map(|path| view! { <TrashBody path=path confirm=confirm /> })}
            </DialogPortal>
        </Dialog>
    }
}

#[component]
fn TrashBody(path: String, confirm: TrashConfirm) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let working = RwSignal::new(false);
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    // Monitors showing it now: hyprpaper keeps it on screen until something
    // else is applied, but it won't come back after a restart.
    let key = path.clone();
    let shown_on = Signal::derive(move || {
        ctx.state.with(|s| {
            s.current
                .iter()
                .filter(|(_, p)| p.to_string_lossy() == key)
                .map(|(m, _)| m.clone())
                .collect::<Vec<_>>()
        })
    });
    let favorite_or_tagged = {
        let p = path.clone();
        move || {
            ctx.catalog
                .with(|c| c.entry(Path::new(&p)).is_some())
        }
    };
    let file = StoredValue::new(path);
    let trash = move || {
        if working.get_untracked() {
            return;
        }
        working.set(true);
        let path = file.get_value();
        spawn_local(async move {
            match api::trash_wallpaper(&path).await {
                Ok(()) => {
                    confirm.open.set(false);
                    let name = Path::new(&path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or(path);
                    ctx.info(format!("Moved {name} to the trash"));
                    ctx.library_rev.update(|r| *r += 1);
                    ctx.refresh_catalog().await;
                    ctx.refresh_state().await;
                }
                Err(e) => ctx.error_with("Could not move it to the trash", &e),
            }
            working.try_set(false);
        });
    };

    view! {
        <DialogHeader>
            <DialogTitle>"Move to trash?"</DialogTitle>
            <DialogDescription class="break-all">{name}</DialogDescription>
        </DialogHeader>
        <div class="flex flex-col gap-2 text-sm text-muted-foreground">
            <p>"You can restore it from your file manager's trash."</p>
            <Show when=favorite_or_tagged.clone()>
                <p>"Its favorite and tags are removed with it."</p>
            </Show>
            <Show when=move || shown_on.with(|m| !m.is_empty())>
                <p>
                    {move || {
                        format!(
                            "It's on {} right now: it stays there until you apply something else, \
                             but won't come back after a restart.",
                            shown_on.get().join(", "),
                        )
                    }}
                </p>
            </Show>
        </div>
        <DialogFooter class="flex gap-2">
            <Button variant=ButtonVariant::Ghost on_click=move |_| confirm.open.set(false)>
                "Cancel"
            </Button>
            <Button variant=ButtonVariant::Destructive disabled=working on_click=move |_| trash()>
                "Move to trash"
            </Button>
        </DialogFooter>
    }
}
