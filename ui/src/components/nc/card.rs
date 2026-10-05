use leptos::prelude::*;
use nocomponents::cn;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum CardSize {
    #[default]
    Default,
    Sm,
}

impl CardSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardSize::Default => "default",
            CardSize::Sm => "sm",
        }
    }
}

#[component]
pub fn Card(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] size: CardSize,
    children: Children,
) -> impl IntoView {
    let size_str = size.as_str();

    view! {
        <div
            data-slot="card"
            data-size=size_str
            class=move || {
                cn!(
                    "group/card flex flex-col gap-4 overflow-hidden rounded-2xl bg-card py-4 text-sm text-card-foreground ring-1 ring-foreground/10 has-data-[slot=card-footer]:pb-0 has-[>img:first-child]:pt-0 data-[size=sm]:gap-3 data-[size=sm]:py-3 data-[size=sm]:has-data-[slot=card-footer]:pb-0 *:[img:first-child]:rounded-t-xl *:[img:last-child]:rounded-b-xl",
                &class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardHeader(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-header"
            class=move || {
                cn!(
                    "group/card-header @container/card-header grid auto-rows-min items-start gap-1 rounded-t-xl px-4 group-data-[size=sm]/card:px-3 has-data-[slot=card-action]:grid-cols-[1fr_auto] has-data-[slot=card-description]:grid-rows-[auto_auto] [.border-b]:pb-4 group-data-[size=sm]/card:[.border-b]:pb-3",
                &class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardTitle(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-title"
            class=move || {
                cn!(
                    "cn-font-heading text-base leading-snug font-medium group-data-[size=sm]/card:text-sm",
                &class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardDescription(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-description"
            class=move || cn!("text-sm text-muted-foreground", &class.get())
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardAction(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-action"
            class=move || {
                cn!(
                    "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
                &class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-content"
            class=move || cn!("px-4 group-data-[size=sm]/card:px-3", &class.get())
        >
            {children()}
        </div>
    }
}

#[component]
pub fn CardFooter(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="card-footer"
            class=move || {
                cn!(
                    "flex mt-auto items-center rounded-b-xl border-t bg-muted/50 p-4 group-data-[size=sm]/card:p-3",
                &class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}
