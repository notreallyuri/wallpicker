use nocomponents::cn;
use nocomponents::primitives::button::ButtonRoot;
use nocomponents::utils::types::AsClass;
use nocomponents::utils::types::StyledRender;

use leptos::ev;
use leptos::prelude::*;
use nocomponents::deps::leptos_node_ref::AnyNodeRef;
pub use nocomponents::primitives::button::ButtonType;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
    Icon,
    IconSm,
    IconXs,
    IconLg,
}

impl AsClass for ButtonSize {
    fn as_class(&self) -> &'static str {
        match self {
            ButtonSize::Default => {
                "h-9 gap-1.5 px-3.5 has-data-[icon=inline-end]:pr-2.5 has-data-[icon=inline-start]:pl-2.5"
            }
            ButtonSize::Xs => {
                "h-6 gap-1 rounded-lg px-2 text-xs in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3"
            }
            ButtonSize::Sm => {
                "h-7 gap-1 rounded-[10px] px-2.5 text-[0.8rem] in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5"
            }
            ButtonSize::Lg => {
                "h-10 gap-1.5 px-4 has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3"
            }

            ButtonSize::Icon => "size-9",
            ButtonSize::IconLg => "size-10",
            ButtonSize::IconSm => {
                "size-7 rounded-[10px] in-data-[slot=button-group]:rounded-lg"
            }
            ButtonSize::IconXs => {
                "size-6 rounded-lg in-data-[slot=button-group]:rounded-lg [&_svg:not([class*='size-'])]:size-3"
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Ghost,
    Secondary,
    Link,
    Destructive,
}

impl AsClass for ButtonVariant {
    fn as_class(&self) -> &'static str {
        match self {
            ButtonVariant::Default => "chunky bg-primary text-primary-foreground [--chunky:color-mix(in_oklab,var(--primary)_55%,black)] hover:brightness-105",
            ButtonVariant::Outline => {
                "chunky border-border bg-card hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:border-input dark:bg-card/80 dark:hover:bg-muted"
            }
            ButtonVariant::Ghost => {
                "hover:bg-primary/12 hover:text-foreground aria-expanded:bg-primary/12 aria-expanded:text-foreground"
            }
            ButtonVariant::Secondary => {
                "chunky bg-secondary text-secondary-foreground [--chunky:color-mix(in_oklab,var(--secondary)_60%,black)] hover:brightness-105 aria-expanded:bg-secondary aria-expanded:text-secondary-foreground"
            }
            ButtonVariant::Link => "text-primary underline-offset-4 hover:underline",
            ButtonVariant::Destructive => {
                "chunky bg-destructive text-white [--chunky:color-mix(in_oklab,var(--destructive)_55%,black)] hover:brightness-105 focus-visible:border-destructive/40 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40"
            }
        }
    }
}

#[component]
pub fn Button(
    #[prop(optional)] node_ref: Option<AnyNodeRef>,
    #[prop(optional, into)] on_click: Option<Callback<ev::MouseEvent>>,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] r#type: ButtonType,
    #[prop(default = None, into)] render: Option<Callback<StyledRender, AnyView>>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let merged_classes = Signal::derive(move || {
        cn!(
            "group/button inline-flex shrink-0 items-center justify-center rounded-xl border border-transparent bg-clip-padding text-sm font-bold whitespace-nowrap transition-all duration-150 ease-bounce outline-none select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 active:not-aria-[haspopup]:translate-y-px disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
            variant.as_class(),
            size.as_class(),
            class.get()
        )
    });

    view! {
        <ButtonRoot
            node_ref=node_ref
            on_click=on_click
            class=merged_classes
            disabled=disabled
            r#type=r#type
            render=render
            children=children
        />
    }
}
