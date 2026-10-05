//! Size filters: a minimum (a preset or any size typed in), and for
//! Wallhaven exact resolutions and aspect ratios, several at once. Nothing is
//! picked for you: a small wallpaper for a 720p or 4:3 screen is a fine find.
//! Your monitors' sizes are only marked among the choices.

use leptos::prelude::*;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use wallpicker_core::wallhaven::{self, is_valid_resolution};

use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::views::Choice;

use super::MIN_RES;

/// "1920x1080" -> "1920×1080".
fn pretty(size: &str) -> String {
    size.replace('x', "×")
}

/// A text field that hands a valid WIDTHxHEIGHT to `add` on Enter (or the
/// button), then clears.
#[component]
fn SizeEntry(#[prop(into)] placeholder: String, #[prop(into)] button: String, add: Callback<String>) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let bad = RwSignal::new(false);
    let submit = move || {
        let v = draft.get_untracked().trim().to_lowercase().replace(['×', ' '], "x");
        if is_valid_resolution(&v) {
            add.run(v);
            draft.set(String::new());
            bad.set(false);
        } else {
            bad.set(!v.is_empty());
        }
    };
    view! {
        <form
            class="flex items-center gap-2"
            on:submit=move |e| {
                e.prevent_default();
                submit();
            }
        >
            <Input model=draft class="h-8 w-44" attr:placeholder=placeholder.clone() attr:aria-invalid=move || bad.get().to_string() />
            <Button size=ButtonSize::Sm variant=ButtonVariant::Outline attr:r#type="submit">
                {button.clone()}
            </Button>
        </form>
    }
}

/// Minimum size: a preset, or any size typed in (shown as the choice then).
#[component]
pub(super) fn MinSize(model: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-wrap items-center gap-2">
            <Choice items=MIN_RES.to_vec() model=model class="w-44" />
            <SizeEntry placeholder="or e.g. 1700x900" button="Set" add=Callback::new(move |v| model.set(v)) />
        </div>
    }
}

/// Wallhaven's exact resolutions: the presets, your monitors' sizes and any
/// typed in, as chips; several at once.
#[component]
pub(super) fn ExactSizes(model: RwSignal<Vec<String>>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    // (size, label): your monitors' first (named), then the presets, then
    // any other picked size.
    let chips = Memo::new(move |_| {
        let mut out: Vec<(String, String)> = Vec::new();
        ctx.monitors.with(|ms| {
            for m in ms {
                let size = format!("{}x{}", m.width, m.height);
                match out.iter_mut().find(|(s, _)| *s == size) {
                    Some((_, label)) => label.push_str(&format!(", {}", m.name)),
                    None => out.push((size.clone(), format!("{} · {}", pretty(&size), m.name))),
                }
            }
        });
        for (size, label) in wallhaven::RESOLUTIONS {
            if !out.iter().any(|(s, _)| s == size) {
                out.push((size.to_string(), label.to_string()));
            }
        }
        model.with(|picked| {
            for size in picked {
                if !out.iter().any(|(s, _)| s == size) {
                    out.push((size.clone(), pretty(size)));
                }
            }
        });
        out
    });
    view! {
        <div class="flex flex-col gap-2">
            <ToggleGroup
                kind=ToggleGroupType::Multiple
                variant=ToggleVariant::Outline
                size=ToggleSize::Sm
                value=model
                class="flex-wrap gap-1.5"
            >
                <For
                    each=move || chips.get()
                    key=|(size, label)| (size.clone(), label.clone())
                    children=|(size, label): (String, String)| {
                        view! {
                            <ToggleGroupItem value=size class="rounded-full! px-3">
                                {label}
                            </ToggleGroupItem>
                        }
                    }
                />
            </ToggleGroup>
            <SizeEntry
                placeholder="other, e.g. 1024x768"
                button="Add"
                add=Callback::new(move |v: String| {
                    model.update(|m| {
                        if !m.contains(&v) {
                            m.push(v);
                        }
                    })
                })
            />
        </div>
    }
}

/// Wallhaven's aspect ratios as chips; several at once, none for any.
#[component]
pub(super) fn Ratios(model: RwSignal<Vec<String>>) -> impl IntoView {
    view! {
        <ToggleGroup
            kind=ToggleGroupType::Multiple
            variant=ToggleVariant::Outline
            size=ToggleSize::Sm
            value=model
            class="flex-wrap gap-1.5"
        >
            {wallhaven::RATIOS
                .iter()
                .map(|(value, label)| {
                    view! {
                        <ToggleGroupItem value=*value class="rounded-full! px-3">
                            {*label}
                        </ToggleGroupItem>
                    }
                })
                .collect_view()}
        </ToggleGroup>
    }
}
