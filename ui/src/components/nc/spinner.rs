use leptos::either::Either;
use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::icons::loader::Loader;
use nocomponents::icons::loader::LoaderCircle;

#[derive(Default, Clone, Copy)]
pub enum SpinnerVariant {
    #[default]
    Default,
    Lined,
}

#[derive(Default, Clone, Copy)]
pub enum SpinnerSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
    Icon,
}

impl SpinnerSize {
    pub fn as_class(&self) -> &'static str {
        match self {
            SpinnerSize::Xs => "size-3",
            SpinnerSize::Sm => "size-3.5",
            SpinnerSize::Default => "size-4",
            SpinnerSize::Lg => "size-5",
            SpinnerSize::Icon => "size-4",
        }
    }
}

#[component]
pub fn Spinner(
    #[prop(optional)] variant: SpinnerVariant,
    #[prop(optional)] size: SpinnerSize,
    #[prop(optional, into)] class: Signal<String>,
) -> impl IntoView {
    let size_class = size.as_class();

    match variant {
        SpinnerVariant::Default => Either::Left(view! {
            <LoaderCircle
                attr:data-slot="spinner"
                class=move || { cn!("size-4 animate-spin", size_class, class.get()) }
            />
        }),
        SpinnerVariant::Lined => Either::Right(view! {
            <Loader
                attr:data-slot="spinner"
                class=move || { cn!("size-4 animate-spin", size_class, class.get()) }
            />
        }),
    }
}
