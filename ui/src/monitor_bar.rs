//! "Apply to", in the top bar: which monitors a click applies to, each a
//! pill with a little screen showing its wallpaper (the file's name in its
//! tooltip), and Previous.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use wallpicker_core::monitors::Monitor;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::{ALL_MONITORS, Ctx};
use crate::views::local::folders::Lucide;

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
        <div class="ml-auto flex flex-wrap items-center gap-2">
            <span class="text-[11px] font-extrabold tracking-wider text-muted-foreground uppercase">"Apply to"</span>
            <Show
                when=move || ctx.monitors.with(|m| !m.is_empty())
                fallback=|| {
                    view! { <span class="text-sm text-muted-foreground">"No monitors found."</span> }
                }
            >
                <ToggleGroup kind=ToggleGroupType::Single value=ctx.selection class="gap-1.5">
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
                            // A Memo: it's used twice (text and tooltip).
                            let current = Memo::new(move |_| {
                                ctx.state
                                    .with(|s| {
                                        s.current
                                            .get(&name)
                                            .and_then(|p| p.file_name())
                                            .map(|f| f.to_string_lossy().into_owned())
                                    })
                                    .unwrap_or_else(|| "nothing set".into())
                            });
                            let size = format!("{}×{}", m.width, m.height);
                            let label = m.name.clone();
                            // The name, its size, and the file it's showing.
                            let tip = move || format!("{label} · {size}\nShowing {}", current.get());
                            view! {
                                <ToggleGroupItem value=m.name.clone() class=CARD attr:title=tip>
                                    <MiniScreen path=shown width=m.width height=m.height />
                                    <span class="font-extrabold">{m.name.clone()}</span>
                                    <Picked />
                                </ToggleGroupItem>
                            }
                        }
                    />
                    <Show when=move || ctx.monitors.with(|m| m.len() > 1)>
                        <ToggleGroupItem value=ALL_MONITORS class=CARD attr:title="Every monitor">
                            // Two little screens side by side, on one stand.
                            <span class="flex shrink-0 flex-col items-center" aria-hidden="true">
                                <span class="flex h-5 items-end gap-0.5">
                                    <span class="block h-4 w-6 rounded-[4px] bg-gradient-to-br from-primary/60 to-primary/25 ring-[1.5px] ring-foreground/25" />
                                    <span class="block h-4 w-6 rounded-[4px] bg-gradient-to-br from-primary/40 to-primary/15 ring-[1.5px] ring-foreground/25" />
                                </span>
                                <span class="block h-0.5 w-1.5 bg-foreground/25" />
                                <span class="block h-0.5 w-7 rounded-full bg-foreground/25" />
                            </span>
                            <span class="font-extrabold">"All"</span>
                            <Picked />
                        </ToggleGroupItem>
                    </Show>
                </ToggleGroup>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Icon
                    attr:aria-label="Previous wallpaper"
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
                    <Lucide class="size-4">
                        <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                        <path d="M3 3v5h5" />
                        <path d="M12 7v5l4 2" />
                    </Lucide>
                </Button>
            </Show>
        </div>
    }
}

/// A monitor's (or All's) pill in the bar: soft, raised when picked; as tall
/// as the buttons beside it.
const CARD: &str = "group/mon h-9 gap-2 rounded-xl border-2 border-transparent bg-card/60 py-0 pr-3 pl-1.5 shadow-sm ring-1 ring-foreground/5 hover:bg-card data-[state=on]:border-primary data-[state=on]:bg-primary/15 data-[state=on]:shadow-[0_3px_0_0_color-mix(in_oklab,var(--primary)_45%,transparent)]";

/// A check in a bubble, shown on the picked card.
#[component]
fn Picked() -> impl IntoView {
    view! {
        <span class="hidden size-4 items-center justify-center rounded-full bg-primary text-primary-foreground group-data-[state=on]/mon:flex">
            <Lucide class="size-3">
                <path d="M20 6 9 17l-5-5" />
            </Lucide>
        </span>
    }
}

/// A little monitor showing what it shows (its library thumbnail, in the
/// monitor's own proportions), on a stand; an empty screen when wallpicker
/// hasn't set anything there.
#[component]
fn MiniScreen(path: Signal<Option<String>>, width: u32, height: u32) -> impl IntoView {
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
    let ratio = if width > 0 && height > 0 { format!("aspect-ratio: {width} / {height}") } else { "aspect-ratio: 16 / 9".into() };
    view! {
        <span class="flex shrink-0 flex-col items-center" aria-hidden="true">
            <span class="block h-5 overflow-hidden rounded-[4px] bg-muted ring-[1.5px] ring-foreground/25" style=ratio>
                {move || src.get().map(|s| view! { <img src=s alt="" class="size-full object-cover" /> })}
            </span>
            <span class="block h-0.5 w-1.5 bg-foreground/25" />
            <span class="block h-0.5 w-4 rounded-full bg-foreground/25" />
        </span>
    }
}
