use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::utils::types::AsClass;

#[derive(Default, Copy, Clone)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

impl AsClass for BadgeVariant {
    fn as_class(&self) -> &'static str {
        match self {
            BadgeVariant::Default => "bg-primary text-primary-foreground [a&]:hover:bg-primary/90",
            BadgeVariant::Secondary => {
                "bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90"
            }
            BadgeVariant::Destructive => {
                "bg-destructive text-white focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40 [a&]:hover:bg-destructive/90"
            }
            BadgeVariant::Outline => {
                "border-border text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground"
            }
            BadgeVariant::Ghost => "[a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
            BadgeVariant::Link => "text-primary underline-offset-4 [a&]:hover:underline",
        }
    }
}

#[component]
pub fn Badge(
    #[prop(optional)] variant: BadgeVariant,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-slot="badge" class=move || {
            cn!(
                "inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent px-2.5 py-0.5 text-xs font-bold whitespace-nowrap transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&>svg]:pointer-events-none [&>svg]:size-3",
                variant.as_class(),
                class.get()
            )
        }>{children()}</div>
    }
}
