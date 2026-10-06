//! Local's color filter: a button that opens a panel of swatches (the eight
//! hues, then Dark, Light, Warm and Muted). Picking one shows the wallpapers that
//! have that color, the most of it first; picking it again, or "Any color",
//! clears it.
//!
//! In a `Popover`: it closes on a pick, a click outside or Esc.

use leptos::prelude::*;
use wallpicker_core::colors::{HUES, TONES};

use crate::components::nc::prelude::*;
use crate::components::popover::{Popover, PopoverPanel};
use crate::components::tip::Tip;

/// What a swatch looks like (CSS background), by key.
fn swatch(key: &str) -> &'static str {
    match key {
        "red" => "#ef4444",
        "orange" => "#f97316",
        "yellow" => "#eab308",
        "green" => "#22c55e",
        "teal" => "#14b8a6",
        "blue" => "#3b82f6",
        "purple" => "#a855f7",
        "pink" => "#ec4899",
        "dark" => "#1c1917",
        "light" => "#f5f5f4",
        "warm" => "linear-gradient(135deg, #b45309, #f59e0b 55%, #fcd34d)",
        // Muted: a soft grey with a hint of color.
        _ => "linear-gradient(135deg, #a8a29e, #d6d3d1 60%, #fda4af)",
    }
}

fn label(key: &str) -> &'static str {
    HUES.iter().chain(TONES.iter()).find(|(k, _)| *k == key).map_or("", |(_, l)| *l)
}

#[component]
pub(super) fn ColorPicker(color: RwSignal<String>, #[prop(into)] loading: Signal<bool>) -> impl IntoView {
    let open = RwSignal::new(false);
    let pick = move |key: &'static str| {
        color.update(|c| *c = if c == key { String::new() } else { key.to_string() });
        open.set(false);
    };
    let dot = |key: &'static str, size: &'static str| {
        view! {
            <span
                class=format!("block shrink-0 rounded-full ring-2 ring-foreground/15 {size}")
                style=format!("background: {}", swatch(key))
            />
        }
    };

    view! {
        <Popover open=open>
            <Tip text="Show the wallpapers with a color">
            <Button
                variant=ButtonVariant::Outline
                // One width whatever color is picked.
                class="w-24"
                attr:aria-expanded=move || open.get().to_string()
                on_click=move |_| open.update(|o| *o = !*o)
            >
                {move || {
                    let c = color.get();
                    if c.is_empty() {
                        // A little palette.
                        view! {
                            <span class="flex -space-x-1.5" aria-hidden="true">
                                {["red", "yellow", "blue"]
                                    .into_iter()
                                    .map(|k| view! {
                                        <span class="block size-3 rounded-full ring-2 ring-card" style=format!("background: {}", swatch(k)) />
                                    })
                                    .collect_view()}
                            </span>
                            "Color"
                        }
                            .into_any()
                    } else {
                        let key: &'static str = HUES.iter().chain(TONES.iter()).find(|(k, _)| *k == c).map_or("", |(k, _)| *k);
                        view! {
                            {dot(key, "size-3.5")}
                            {label(key)}
                        }
                            .into_any()
                    }
                }}
                <Show when=move || loading.get()>
                    <Spinner class="size-3.5" />
                </Show>
            </Button>
            </Tip>
            <PopoverPanel open=open class="w-72 p-3">
                    <div class="grid grid-cols-4 gap-2">
                        {HUES
                            .iter()
                            .map(|&(key, name)| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=move |_| pick(key)
                                        class=move || {
                                            format!(
                                                "flex flex-col items-center gap-1 rounded-xl p-1.5 text-[11px] font-bold transition duration-200 ease-bounce hover:-translate-y-0.5 hover:bg-primary/10 {}",
                                                if color.with(|c| c == key) { "bg-primary/15 ring-2 ring-primary" } else { "" },
                                            )
                                        }
                                    >
                                        {dot(key, "size-7")}
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <div class="mt-2 grid grid-cols-2 gap-2 border-t-2 border-border/60 pt-2">
                        {TONES
                            .iter()
                            .map(|&(key, name)| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=move |_| pick(key)
                                        class=move || {
                                            format!(
                                                "flex items-center gap-2 rounded-xl px-2 py-1.5 text-xs font-bold transition hover:bg-primary/10 {}",
                                                if color.with(|c| c == key) { "bg-primary/15 ring-2 ring-primary" } else { "" },
                                            )
                                        }
                                    >
                                        {dot(key, "size-4")}
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <Show when=move || color.with(|c| !c.is_empty())>
                        <button
                            type="button"
                            on:click=move |_| {
                                color.set(String::new());
                                open.set(false);
                            }
                            class="mt-2 w-full rounded-xl px-2 py-1.5 text-xs font-bold text-muted-foreground transition hover:bg-primary/10"
                        >
                            "Any color"
                        </button>
                    </Show>
                    <p class="mt-2 px-1 text-[11px] text-muted-foreground">"The most of the color first."</p>
            </PopoverPanel>
        </Popover>
    }
}
