use crate::components::nc::button::ButtonSize;
use crate::components::nc::button::ButtonVariant;
use nocomponents::cn;
use nocomponents::icons::chevron::ChevronLeft;
use nocomponents::icons::chevron::ChevronRight;
use nocomponents::utils::types::AsClass;
use leptos::prelude::*;

fn link_class(active: bool, size: ButtonSize) -> String {
    let variant = if active {
        ButtonVariant::Outline
    } else {
        ButtonVariant::Ghost
    };

    format!(
        "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg border border-transparent bg-clip-padding text-sm font-medium whitespace-nowrap transition-all outline-none select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 {} {}",
        variant.as_class(),
        size.as_class()
    )
}

#[component]
pub fn Pagination(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <nav
            role="navigation"
            aria-label="pagination"
            data-slot="pagination"
            class=move || cn!("mx-auto flex w-full justify-center", class.get())
        >
            {children()}
        </nav>
    }
}

#[component]
pub fn PaginationContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <ul
            data-slot="pagination-content"
            class=move || cn!("flex flex-row items-center gap-1", class.get())
        >
            {children()}
        </ul>
    }
}

#[component]
pub fn PaginationItem(children: Children) -> impl IntoView {
    view! { <li data-slot="pagination-item">{children()}</li> }
}

#[component]
pub fn PaginationLink(
    #[prop(optional, into)] href: Signal<String>,
    #[prop(optional, into)] active: Signal<bool>,
    #[prop(default = ButtonSize::Icon)] size: ButtonSize,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <a
            href=move || href.get()
            data-slot="pagination-link"
            data-active=move || active.get().then_some("")
            aria-current=move || active.get().then_some("page")
            class=move || cn!(link_class(active.get(), size).as_str(), class.get())
        >
            {children()}
        </a>
    }
}

#[component]
pub fn PaginationPrevious(
    #[prop(optional, into)] href: Signal<String>,
    #[prop(optional, into)] class: Signal<String>,
) -> impl IntoView {
    view! {
        <PaginationLink
            href=href
            size=ButtonSize::Default
            class=move || cn!("gap-1 px-2.5 sm:pl-2.5", class.get())
            attr:aria-label="Go to previous page"
        >
            <ChevronLeft />
            <span class="hidden sm:block">"Previous"</span>
        </PaginationLink>
    }
}

#[component]
pub fn PaginationNext(
    #[prop(optional, into)] href: Signal<String>,
    #[prop(optional, into)] class: Signal<String>,
) -> impl IntoView {
    view! {
        <PaginationLink
            href=href
            size=ButtonSize::Default
            class=move || cn!("gap-1 px-2.5 sm:pr-2.5", class.get())
            attr:aria-label="Go to next page"
        >
            <span class="hidden sm:block">"Next"</span>
            <ChevronRight />
        </PaginationLink>
    }
}

#[component]
pub fn PaginationEllipsis(#[prop(optional, into)] class: Signal<String>) -> impl IntoView {
    view! {
        <span
            aria-hidden="true"
            data-slot="pagination-ellipsis"
            class=move || {
                cn!("flex size-8 items-center justify-center text-muted-foreground", class.get())
            }
        >
            "…"
            <span class="sr-only">"More pages"</span>
        </span>
    }
}
