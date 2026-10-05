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
                "h-8 gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2"
            }
            ButtonSize::Xs => {
                "h-6 gap-1 rounded-[min(var(--radius-md),10px)] px-2 text-xs in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3"
            }
            ButtonSize::Sm => {
                "h-7 gap-1 rounded-[min(var(--radius-md),12px)] px-2.5 text-[0.8rem] in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5"
            }
            ButtonSize::Lg => {
                "h-9 gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2"
            }

            ButtonSize::Icon => "size-8",
            ButtonSize::IconLg => "size-9",
            ButtonSize::IconSm => {
                "size-7 rounded-[min(var(--radius-md),12px)] in-data-[slot=button-group]:rounded-lg"
            }
            ButtonSize::IconXs => {
                "size-6 rounded-[min(var(--radius-md),10px)] in-data-[slot=button-group]:rounded-lg [&_svg:not([class*='size-'])]:size-3"
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
            ButtonVariant::Default => "bg-primary text-primary-foreground [a]:hover:bg-primary/80",
            ButtonVariant::Outline => {
                "border-border bg-background hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50"
            }
            ButtonVariant::Ghost => {
                "hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:hover:bg-muted/50"
            }
            ButtonVariant::Secondary => {
                "bg-secondary text-secondary-foreground hover:bg-secondary/80 aria-expanded:bg-secondary aria-expanded:text-secondary-foreground"
            }
            ButtonVariant::Link => "text-primary underline-offset-4 hover:underline",
            ButtonVariant::Destructive => {
                "bg-destructive/10 text-destructive hover:bg-destructive/20 focus-visible:border-destructive/40 focus-visible:ring-destructive/20 dark:bg-destructive/20 dark:hover:bg-destructive/30 dark:focus-visible:ring-destructive/40"
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
            "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding text-sm font-medium whitespace-nowrap transition-all outline-none select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 active:not-aria-[haspopup]:translate-y-px disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
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
