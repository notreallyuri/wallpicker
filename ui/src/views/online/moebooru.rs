//! Filters of the Moebooru providers (Konachan, yande.re): sorting, minimum
//! size and ratings. One form type, one instance per provider.

use leptos::prelude::*;
use wallpicker_core::moebooru::{MoebooruQuery, MoebooruSort, Ratings};
use wallpicker_core::online::Provider;
use wallpicker_core::settings::KonachanSite;

use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::views::Choice;

use super::pairs;
use super::sizes::MinSize;

const SORTINGS: &[(MoebooruSort, &str)] = &[
    (MoebooruSort::Latest, "Latest"),
    (MoebooruSort::Score, "Score"),
    (MoebooruSort::Random, "Random"),
];

#[derive(Clone, Copy)]
pub(super) struct MoebooruForm {
    sorting: RwSignal<String>,
    atleast: RwSignal<String>,
    safe: RwSignal<bool>,
    questionable: RwSignal<bool>,
    explicit: RwSignal<bool>,
}

impl MoebooruForm {
    pub(super) fn new() -> MoebooruForm {
        let form = MoebooruForm {
            sorting: RwSignal::new(String::new()),
            atleast: RwSignal::new(String::new()),
            safe: RwSignal::new(true),
            questionable: RwSignal::new(false),
            explicit: RwSignal::new(false),
        };
        form.restore(MoebooruQuery::default());
        form
    }

    pub(super) fn restore(self, f: MoebooruQuery) {
        self.sorting.set(f.sorting.as_str().to_string());
        self.atleast.set(f.atleast.unwrap_or_default());
        self.safe.set(f.ratings.safe);
        self.questionable.set(f.ratings.questionable);
        self.explicit.set(f.ratings.explicit);
    }

    /// The filters as set (no tags), tracked so an Effect can save them.
    pub(super) fn filters(self) -> MoebooruQuery {
        MoebooruQuery {
            sorting: SORTINGS
                .iter()
                .map(|(s, _)| *s)
                .find(|s| self.sorting.with(|v| s.as_str() == v))
                .unwrap_or_default(),
            atleast: Some(self.atleast.get()).filter(|s| !s.is_empty()),
            ratings: Ratings {
                safe: self.safe.get(),
                questionable: self.questionable.get(),
                explicit: self.explicit.get(),
            },
            ..MoebooruQuery::default()
        }
    }

    pub(super) fn sort_latest(self) {
        self.sorting.set(MoebooruSort::Latest.as_str().to_string());
    }
}

/// Konachan on konachan.net only has Safe posts, so ratings don't apply
/// there (the backend searches Safe whatever is ticked).
pub(super) fn ratings_apply(ctx: Ctx, provider: Provider) -> bool {
    provider != Provider::Konachan
        || ctx.settings.with(|s| {
            s.as_ref()
                .is_some_and(|s| s.konachan_site == KonachanSite::Com)
        })
}

#[component]
pub(super) fn MoebooruFilters(form: MoebooruForm, provider: Provider) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let check = |label: &'static str, checked: RwSignal<bool>| {
        view! {
            <Label class="cursor-pointer font-normal">
                <Checkbox checked=checked />
                {label}
            </Label>
        }
    };
    let note = match provider {
        Provider::Yandere => "yande.re: high-resolution scans, all ratings.",
        _ => "",
    };

    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            <Field>
                <FieldLabel>"Sort by"</FieldLabel>
                <Choice items=pairs(SORTINGS, MoebooruSort::as_str) model=form.sorting />
            </Field>
            <Field>
                <FieldLabel>"Minimum size"</FieldLabel>
                <MinSize model=form.atleast />
            </Field>
        </div>
        <Show
            when=move || ratings_apply(ctx, provider)
            fallback=|| {
                view! {
                    <p class="text-sm text-muted-foreground">
                        "konachan.net only has safe-for-work posts. Switch to konachan.com in "
                        "Settings → Konachan for all ratings."
                    </p>
                }
            }
        >
            <Field>
                <FieldLabel>"Ratings"</FieldLabel>
                <div class="flex flex-wrap gap-x-5 gap-y-2 text-sm">
                    {check("Safe", form.safe)}
                    {check("Questionable", form.questionable)}
                    {check("Explicit", form.explicit)}
                </div>
            </Field>
        </Show>
        <p class="text-sm text-muted-foreground">
            {note} " In the search box: tags, -tag to exclude, user:<name> for an uploader."
        </p>
    }
}
