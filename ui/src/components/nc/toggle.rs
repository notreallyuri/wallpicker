use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::toggle::ToggleRoot;
use nocomponents::utils::types::AsClass;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ToggleVariant {
    #[default]
    Default,
    Outline,
}

impl AsClass for ToggleVariant {
    fn as_class(&self) -> &'static str {
        match self {
            ToggleVariant::Default => "bg-transparent",
            ToggleVariant::Outline => "border-border bg-transparent dark:border-input",
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ToggleSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
}

impl AsClass for ToggleSize {
    fn as_class(&self) -> &'static str {
        match self {
            ToggleSize::Xs => {
                "h-6 min-w-6 gap-1 px-1.5 text-xs [&_svg:not([class*='size-'])]:size-3"
            }
            ToggleSize::Sm => {
                "h-7 min-w-7 gap-1 px-2 text-[0.8rem] [&_svg:not([class*='size-'])]:size-3.5"
            }
            ToggleSize::Default => "h-8 min-w-8 gap-1.5 px-2.5",
            ToggleSize::Lg => "h-9 min-w-9 gap-1.5 px-2.5",
        }
    }
}

pub(crate) const TOGGLE_BASE: &str = "inline-flex shrink-0 items-center justify-center rounded-xl border border-transparent text-sm font-bold transition-all duration-150 ease-bounce active:scale-95 whitespace-nowrap transition-all outline-none select-none data-[state=off]:hover:bg-muted data-[state=off]:hover:text-foreground data-[state=on]:bg-primary/20 data-[state=on]:text-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

#[component]
pub fn Toggle(
    #[prop(optional)] variant: ToggleVariant,
    #[prop(optional)] size: ToggleSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = None, into)] pressed: Option<RwSignal<bool>>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] on_change: Option<Callback<bool>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <ToggleRoot
            pressed=pressed
            disabled=disabled
            on_change=on_change
            class=move || cn!(TOGGLE_BASE, variant.as_class(), size.as_class(), class.get())
        >
            {children.map(|c| c())}
        </ToggleRoot>
    }
}
