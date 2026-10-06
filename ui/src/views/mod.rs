//! The app's tabs.

use leptos::prelude::*;

use crate::components::nc::prelude::*;

pub(crate) mod local;
pub(crate) mod online;
pub(crate) mod settings;
mod workspaces;

/// A nocomponents `Select` over `(value, label)` pairs, bound to a plain
/// `String` model like the other form fields.
///
/// `Select` works on `Option<String>` and its stock `SelectValue` shows the raw
/// value ("toplist"), so this keeps the two in sync and shows the label.
#[component]
pub(crate) fn Choice(
    items: Vec<(&'static str, &'static str)>,
    model: RwSignal<String>,
    #[prop(optional, into)] class: Signal<String>,
) -> impl IntoView {
    let value = RwSignal::new(Some(model.get_untracked()));
    // The app sets some models itself (loading settings, searches started
    // from the details dialog), so sync both ways.
    Effect::new(move |_| {
        let m = model.get();
        if value.with_untracked(|v| v.as_deref() != Some(m.as_str())) {
            value.set(Some(m));
        }
    });
    Effect::new(move |_| {
        if let Some(v) = value.get()
            && model.with_untracked(|m| *m != v)
        {
            model.set(v);
        }
    });

    let items = StoredValue::new(items);
    let label = move || {
        model.with(|m| {
            items.with_value(|items| {
                items
                    .iter()
                    .find(|(v, _)| v == m)
                    .map_or_else(|| m.clone(), |(_, l)| l.to_string())
            })
        })
    };

    view! {
        <Select value=value class=class>
            <SelectTrigger>
                <span class="truncate">{label}</span>
            </SelectTrigger>
            <SelectPortal>
                <SelectContent>
                    {items
                        .get_value()
                        .into_iter()
                        .map(|(v, l)| view! { <SelectItem value=v>{l}</SelectItem> })
                        .collect_view()}
                </SelectContent>
            </SelectPortal>
        </Select>
    }
}

/// A fit by its saved name; "" (or anything else) is none.
pub(crate) fn fit_from(value: &str) -> Option<wallpicker_core::settings::FitMode> {
    use wallpicker_core::settings::FitMode;
    match value {
        "cover" => Some(FitMode::Cover),
        "contain" => Some(FitMode::Contain),
        "tile" => Some(FitMode::Tile),
        "fill" => Some(FitMode::Fill),
        _ => None,
    }
}
