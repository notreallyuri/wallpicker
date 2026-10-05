use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::utils::types::AsClass;

#[derive(Default, Clone, Copy)]
pub enum EmptyMediaVariant {
    #[default]
    Icon,
    Plain,
}

impl AsClass for EmptyMediaVariant {
    fn as_class(&self) -> &'static str {
        match self {
            EmptyMediaVariant::Icon => {
                "flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-foreground [&_svg:not([class*='size-'])]:size-5"
            }
            EmptyMediaVariant::Plain => "bg-transparent",
        }
    }
}

#[component]
pub fn Empty(#[prop(optional, into)] class: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <div
            data-slot="empty"
            class=move || {
                cn!(
                    "flex min-w-0 flex-1 flex-col items-center justify-center gap-6 rounded-lg border border-dashed p-6 text-center text-balance md:p-12",
                    class.get()
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn EmptyHeader(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="empty-header"
            class=move || {
                cn!("flex max-w-sm flex-col items-center gap-2 text-center", class.get())
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn EmptyMedia(
    #[prop(optional)] variant: EmptyMediaVariant,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    let variant_class = variant.as_class();

    view! {
        <div
            data-slot="empty-media"
            data-variant=match variant {
                EmptyMediaVariant::Icon => "icon",
                EmptyMediaVariant::Plain => "plain",
            }
            class=move || {
                cn!("mb-2 flex items-center justify-center", variant_class, class.get())
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn EmptyTitle(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="empty-title"
            class=move || cn!("text-lg font-medium tracking-tight", class.get())
        >
            {children()}
        </div>
    }
}

#[component]
pub fn EmptyDescription(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="empty-description"
            class=move || {
                cn!(
                    "text-sm/relaxed text-muted-foreground [&>a]:underline [&>a]:underline-offset-4 [&>a:hover]:text-primary",
                    class.get()
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn EmptyContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="empty-content"
            class=move || {
                cn!(
                    "flex w-full max-w-sm min-w-0 flex-col items-center gap-4 text-sm text-balance",
                    class.get()
                )
            }
        >
            {children()}
        </div>
    }
}
