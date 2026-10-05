//! The "Apply to" bar: which monitors a click applies to.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use wallpicker_core::monitors::Monitor;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::{ALL_MONITORS, Ctx};

#[component]
pub(crate) fn MonitorBar() -> impl IntoView {
    let ctx = expect_context::<Ctx>();

    // A single-select ToggleGroup clears its value when the active item is
    // clicked again. Put the previous choice back so something is always
    // selected.
    Effect::new(move |prev: Option<Vec<String>>| {
        let current = ctx.selection.get();
        match prev {
            Some(prev) if current.is_empty() && !prev.is_empty() => {
                ctx.selection.set(prev.clone());
                prev
            }
            _ => current,
        }
    });

    view! {
        <div class="flex flex-wrap items-center gap-3 border-b bg-muted/30 px-5 py-2.5">
            <span class="text-sm text-muted-foreground">"Apply to"</span>
            <Show
                when=move || ctx.monitors.with(|m| !m.is_empty())
                fallback=|| {
                    view! { <span class="text-sm text-muted-foreground">"No monitors found."</span> }
                }
            >
                <ToggleGroup
                    kind=ToggleGroupType::Single
                    variant=ToggleVariant::Outline
                    value=ctx.selection
                >
                    <For
                        each=move || ctx.monitors.get()
                        key=|m| m.name.clone()
                        children=move |m: Monitor| {
                            let name = m.name.clone();
                            let shown_name = m.name.clone();
                            let shown = Signal::derive(move || {
                                ctx.state.with(|s| {
                                    s.current.get(&shown_name).map(|p| p.to_string_lossy().into_owned())
                                })
                            });
                            let current = move || {
                                ctx.state
                                    .with(|s| {
                                        s.current
                                            .get(&name)
                                            .and_then(|p| p.file_name())
                                            .map(|f| f.to_string_lossy().into_owned())
                                    })
                                    .unwrap_or_else(|| "nothing set".into())
                            };
                            view! {
                                <ToggleGroupItem value=m.name.clone() class="h-auto gap-2 py-1 pl-1 pr-3">
                                    <NowShowing path=shown />
                                    <span class="font-medium">{m.name.clone()}</span>
                                    <span class="text-xs text-muted-foreground">
                                        {format!("{}×{}", m.width, m.height)}
                                    </span>
                                    <span class="max-w-40 truncate text-xs text-muted-foreground">
                                        {current}
                                    </span>
                                </ToggleGroupItem>
                            }
                        }
                    />
                    <Show when=move || ctx.monitors.with(|m| m.len() > 1)>
                        // As tall as the monitors (their thumbnail is h-7).
                        <ToggleGroupItem value=ALL_MONITORS class="h-auto py-1 px-3">
                            <span class="flex h-7 items-center font-medium">"All"</span>
                        </ToggleGroupItem>
                    </Show>
                </ToggleGroup>
                <Button
                    class="ml-auto"
                    variant=ButtonVariant::Ghost
                    attr:title="Back to the wallpaper before, on the selected monitors"
                    on_click=move |_| {
                        let targets = ctx.targets.get_untracked();
                        spawn_local(async move {
                            match api::previous_wallpaper(&targets).await {
                                Ok(r) if !r.applied.is_empty() => ctx.info(format!("Back on {}", r.applied.join(", "))),
                                Ok(_) => ctx.info("Nothing earlier to go back to"),
                                Err(e) => ctx.error_with("Could not go back", &e),
                            }
                            ctx.refresh_state().await;
                        });
                    }
                >
                    "↶ Previous"
                </Button>
            </Show>
        </div>
    }
}

/// A small picture of what a monitor shows (its library thumbnail), or an
/// empty box when wallpicker hasn't set anything there.
#[component]
fn NowShowing(path: Signal<Option<String>>) -> impl IntoView {
    let src = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let Some(p) = path.get() else {
            src.set(None);
            return;
        };
        spawn_local(async move {
            // A file that's gone (trashed, moved) just shows no picture.
            let url = api::thumbnail(&p).await.ok().map(|t| api::asset_url(&t));
            src.try_set(url);
        });
    });
    view! {
        <span class="block h-7 w-12 shrink-0 overflow-hidden rounded bg-muted">
            {move || src.get().map(|s| view! { <img src=s alt="" class="size-full object-cover" /> })}
        </span>
    }
}
