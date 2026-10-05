//! Wallhaven's filters: sorting, top range, sizes, ratios, categories, purity.

use leptos::prelude::*;
use wallpicker_core::wallhaven::{self, Categories, Purity, SearchQuery, Sorting, TopRange};

use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::views::Choice;

use super::pairs;
use super::sizes::{ExactSizes, MinSize, Ratios};

const SORTINGS: &[(Sorting, &str)] = &[
    (Sorting::Toplist, "Top"),
    (Sorting::Hot, "Hot"),
    (Sorting::DateAdded, "Latest"),
    (Sorting::Relevance, "Relevance"),
    (Sorting::Views, "Views"),
    (Sorting::Favorites, "Favorites"),
    (Sorting::Random, "Random"),
];

const RANGES: &[(TopRange, &str)] = &wallhaven::TOP_RANGES;

/// The form's controls. Each gets its own signal, which is what the
/// nocomponents controls bind to.
#[derive(Clone, Copy)]
pub(super) struct WallhavenForm {
    sorting: RwSignal<String>,
    range: RwSignal<String>,
    atleast: RwSignal<String>,
    resolutions: RwSignal<Vec<String>>,
    ratios: RwSignal<Vec<String>>,
    general: RwSignal<bool>,
    anime: RwSignal<bool>,
    people: RwSignal<bool>,
    sfw: RwSignal<bool>,
    sketchy: RwSignal<bool>,
    nsfw: RwSignal<bool>,
}

impl WallhavenForm {
    pub(super) fn new() -> WallhavenForm {
        let form = WallhavenForm {
            sorting: RwSignal::new(String::new()),
            range: RwSignal::new(String::new()),
            atleast: RwSignal::new(String::new()),
            resolutions: RwSignal::new(Vec::new()),
            ratios: RwSignal::new(Vec::new()),
            general: RwSignal::new(true),
            anime: RwSignal::new(true),
            people: RwSignal::new(true),
            sfw: RwSignal::new(true),
            sketchy: RwSignal::new(false),
            nsfw: RwSignal::new(false),
        };
        form.restore(SearchQuery::default());
        form
    }

    pub(super) fn restore(self, f: SearchQuery) {
        self.sorting.set(f.sorting.as_str().to_string());
        self.range
            .set(f.top_range.unwrap_or(TopRange::OneMonth).as_str().to_string());
        self.atleast.set(f.atleast.unwrap_or_default());
        self.resolutions.set(f.resolutions);
        self.ratios.set(f.ratios);
        self.general.set(f.categories.general);
        self.anime.set(f.categories.anime);
        self.people.set(f.categories.people);
        self.sfw.set(f.purity.sfw);
        self.sketchy.set(f.purity.sketchy);
        self.nsfw.set(f.purity.nsfw);
    }

    /// The filters as set (no search text), tracked so an Effect can save
    /// them. NSFW is kept as ticked; the search drops it without an API key.
    pub(super) fn filters(self) -> SearchQuery {
        let sorting = SORTINGS
            .iter()
            .map(|(s, _)| *s)
            .find(|s| self.sorting.with(|v| s.as_str() == v))
            .unwrap_or(Sorting::DateAdded);
        let top_range = RANGES
            .iter()
            .map(|(r, _)| *r)
            .find(|r| self.range.with(|v| r.as_str() == v));
        SearchQuery {
            categories: Categories {
                general: self.general.get(),
                anime: self.anime.get(),
                people: self.people.get(),
            },
            purity: Purity {
                sfw: self.sfw.get(),
                sketchy: self.sketchy.get(),
                nsfw: self.nsfw.get(),
            },
            sorting,
            top_range,
            atleast: Some(self.atleast.get()).filter(|s| !s.is_empty()),
            resolutions: self.resolutions.get(),
            ratios: self.ratios.get(),
            ..SearchQuery::default()
        }
    }

    /// Tag and uploader searches are most useful newest first; "Top over
    /// the last month" would hide most of them.
    pub(super) fn sort_latest(self) {
        self.sorting.set(Sorting::DateAdded.as_str().to_string());
    }
}

#[component]
pub(super) fn WallhavenFilters(form: WallhavenForm) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let no_key = Signal::derive(move || !ctx.has_api_key());
    let check = |label: &'static str, checked: RwSignal<bool>, disabled: Signal<bool>| {
        view! {
            <Label class="cursor-pointer font-normal">
                <Checkbox checked=checked disabled=disabled />
                {label}
            </Label>
        }
    };

    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            <Field>
                <FieldLabel>"Sort by"</FieldLabel>
                <Choice items=pairs(SORTINGS, Sorting::as_str) model=form.sorting />
            </Field>
            <Show when=move || form.sorting.get() == Sorting::Toplist.as_str()>
                <Field>
                    <FieldLabel>"Top over"</FieldLabel>
                    <Choice items=pairs(RANGES, TopRange::as_str) model=form.range />
                </Field>
            </Show>
        </div>
        <Field>
            <FieldLabel>"Minimum size"</FieldLabel>
            <MinSize model=form.atleast />
            <Show when=move || form.resolutions.with(|r| !r.is_empty()) && !form.atleast.with(String::is_empty)>
                <FieldDescription>"Not used while exact resolutions are picked (Wallhaven takes one or the other)."</FieldDescription>
            </Show>
        </Field>
        <Field>
            <FieldLabel>"Exact resolutions"</FieldLabel>
            <ExactSizes model=form.resolutions />
            <FieldDescription>"Any number; none means any. Your monitors' sizes come first."</FieldDescription>
        </Field>
        <Field>
            <FieldLabel>"Aspect ratios"</FieldLabel>
            <Ratios model=form.ratios />
        </Field>
        <Field>
            <FieldLabel>"Categories"</FieldLabel>
            <div class="flex flex-wrap gap-x-5 gap-y-2 text-sm">
                {check("General", form.general, Signal::from(false))}
                {check("Anime", form.anime, Signal::from(false))}
                {check("People", form.people, Signal::from(false))}
            </div>
        </Field>
        <Field>
            <FieldLabel>"Purity"</FieldLabel>
            <div class="flex flex-wrap gap-x-5 gap-y-2 text-sm">
                {check("SFW", form.sfw, Signal::from(false))}
                {check("Sketchy", form.sketchy, Signal::from(false))}
                {check("NSFW", form.nsfw, no_key)}
            </div>
            <Show when=move || no_key.get()>
                <FieldDescription>"NSFW needs an API key (Settings → Wallhaven)."</FieldDescription>
            </Show>
        </Field>
    }
}
