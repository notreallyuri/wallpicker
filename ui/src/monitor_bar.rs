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
use crate::components::tip::Tip;

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
                            let tip = Signal::derive(move || format!("{label} · {size}\nShowing {}", current.get()));
                            // Copyable: the tooltip may draw its contents again.
                            let name = StoredValue::new(m.name.clone());
                            let (width, height) = (m.width, m.height);
                            view! {
                                <Tip text=tip>
                                    <ToggleGroupItem value=name.get_value() class=CARD>
                                        <MiniScreen path=shown width=width height=height />
                                        <span class="font-extrabold">{name.get_value()}</span>
                                        <Picked />
                                    </ToggleGroupItem>
                                </Tip>
                            }
                        }
                    />
                    <Show when=move || ctx.monitors.with(|m| m.len() > 1)>
                        <Tip text="Every monitor">
                        <ToggleGroupItem value=ALL_MONITORS class=CARD>
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
                        </Tip>
                    </Show>
                </ToggleGroup>
                <Tip text=Signal::derive(move || {
                    if ctx.monitors_open.get() { "Hide the monitors" } else { "Show the monitors, bigger" }.to_string()
                })>
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::IconSm
                        attr:aria-label="Show the monitors"
                        attr:aria-expanded=move || ctx.monitors_open.get().to_string()
                        on_click=move |_| ctx.monitors_open.update(|o| *o = !*o)
                    >
                        <span class=move || {
                            format!(
                                "flex transition-transform duration-300 ease-bounce {}",
                                if ctx.monitors_open.get() { "rotate-180" } else { "" },
                            )
                        }>
                            <Lucide class="size-4">
                                <path d="m6 9 6 6 6-6" />
                            </Lucide>
                        </span>
                    </Button>
                </Tip>
                <Tip text="Back to the wallpaper before, on the selected monitors">
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Icon
                    attr:aria-label="Previous wallpaper"
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
                </Tip>
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
fn MiniScreen(
    path: Signal<Option<String>>,
    width: u32,
    height: u32,
    /// The drawer's big screen instead of the pill's little one.
    #[prop(optional)]
    big: bool,
) -> impl IntoView {
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
    let (screen, neck, base) = if big {
        ("block h-28 overflow-hidden rounded-xl bg-muted shadow-md ring-[3px] ring-foreground/20", "block h-2 w-4 bg-foreground/20", "block h-1 w-16 rounded-full bg-foreground/20")
    } else {
        ("block h-5 overflow-hidden rounded-[4px] bg-muted ring-[1.5px] ring-foreground/25", "block h-0.5 w-1.5 bg-foreground/25", "block h-0.5 w-4 rounded-full bg-foreground/25")
    };
    view! {
        <span class="flex shrink-0 flex-col items-center" aria-hidden="true">
            <span class=screen style=ratio>
                {move || src.get().map(|s| view! { <img src=s alt="" class="size-full object-cover" /> })}
            </span>
            <span class=neck />
            <span class=base />
        </span>
    }
}

/// Under the top bar, opened by its chevron: each monitor bigger, with what
/// it shows. A card picks its monitor as the target, like the pills; its
/// ↶ goes back on that monitor only. It slides open and pushes the tab
/// below down (a grid row from 0fr to 1fr, so the height animates).
#[component]
pub(crate) fn MonitorDrawer() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    Effect::new(move |_| crate::prefs::set("ui.monitors_open", if ctx.monitors_open.get() { "1" } else { "0" }));
    let previous = move |monitor: String| {
        spawn_local(async move {
            match api::previous_wallpaper(&[monitor.clone()]).await {
                Ok(r) if !r.applied.is_empty() => ctx.info(format!("Back on {monitor}")),
                Ok(_) => ctx.info(format!("Nothing earlier on {monitor}")),
                Err(e) => ctx.error_with("Could not go back", &e),
            }
            ctx.refresh_state().await;
        });
    };
    view! {
        <div class=move || {
            format!(
                "grid transition-[grid-template-rows] duration-300 ease-out {}",
                if ctx.monitors_open.get() && ctx.monitors.with(|m| !m.is_empty()) { "grid-rows-[1fr]" } else { "grid-rows-[0fr]" },
            )
        }>
            <div class="min-h-0 overflow-hidden">
                <div class="flex flex-wrap justify-center gap-4 border-b border-border/60 bg-background/45 px-5 py-4 backdrop-blur-xl">
                    <For
                        each=move || ctx.monitors.get()
                        key=|m| m.name.clone()
                        children=move |m: Monitor| {
                            let name = StoredValue::new(m.name.clone());
                            let shown = Signal::derive(move || {
                                ctx.state.with(|s| s.current.get(&name.get_value()).map(|p| p.to_string_lossy().into_owned()))
                            });
                            let file = move || {
                                shown
                                    .get()
                                    .and_then(|p| p.rsplit('/').next().map(str::to_string))
                                    .unwrap_or_else(|| "nothing set".into())
                            };
                            let picked = move || {
                                ctx.selection.with(|s| s.iter().any(|v| *v == name.get_value() || v == ALL_MONITORS))
                            };
                            view! {
                                <div
                                    role="button"
                                    tabindex="0"
                                    on:click=move |_| ctx.selection.set(vec![name.get_value()])
                                    class=move || {
                                        format!(
                                            "flex w-72 cursor-pointer flex-col items-center gap-2 rounded-3xl border-2 p-3 transition duration-300 ease-bounce hover:-translate-y-0.5 {}",
                                            if picked() {
                                                "border-primary bg-primary/12 shadow-[0_3px_0_0_color-mix(in_oklab,var(--primary)_45%,transparent)]"
                                            } else {
                                                "border-transparent bg-card/60 hover:bg-card"
                                            },
                                        )
                                    }
                                >
                                    <MiniScreen path=shown width=m.width height=m.height big=true />
                                    <div class="flex w-full items-center gap-2">
                                        <div class="flex min-w-0 flex-1 flex-col leading-tight">
                                            <span class="flex items-center gap-1.5 font-extrabold">
                                                {name.get_value()}
                                                <span class="text-xs font-semibold text-muted-foreground">{format!("{}×{}", m.width, m.height)}</span>
                                            </span>
                                            <span class="truncate text-xs text-muted-foreground">{file}</span>
                                        </div>
                                        <Tip text=Signal::derive(move || format!("Back to the wallpaper before, on {}", name.get_value()))>
                                            <Button
                                                variant=ButtonVariant::Ghost
                                                size=ButtonSize::IconSm
                                                attr:aria-label="Previous wallpaper on this monitor"
                                                on_click=move |e: leptos::ev::MouseEvent| {
                                                    // Not a click on the card.
                                                    e.stop_propagation();
                                                    previous(name.get_value());
                                                }
                                            >
                                                <Lucide class="size-4">
                                                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                                    <path d="M3 3v5h5" />
                                                </Lucide>
                                            </Button>
                                        </Tip>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}
