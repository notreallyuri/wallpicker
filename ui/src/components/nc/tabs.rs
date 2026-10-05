use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::tabs::TabsContentRoot;
use nocomponents::primitives::tabs::TabsListRoot;
use nocomponents::primitives::tabs::TabsListVariants;
use nocomponents::primitives::tabs::TabsRoot;
use nocomponents::primitives::tabs::TabsTriggerRoot;
use nocomponents::utils::types::Orientation;

fn list_variant_class(variant: TabsListVariants) -> &'static str {
    match variant {
        TabsListVariants::Default => "bg-muted",
        TabsListVariants::Line => "gap-1 bg-transparent",
    }
}

#[component]
pub fn Tabs(
    #[prop(into)] default_value: String,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] orientation: Orientation,
    children: Children,
) -> impl IntoView {
    view! {
        <TabsRoot
            default_value=default_value
            orientation=orientation
            class=move || cn!("group/tabs flex gap-2 data-horizontal:flex-col", class.get())
        >
            {children()}
        </TabsRoot>
    }
}

#[component]
pub fn TabsList(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] variant: TabsListVariants,
    children: Children,
) -> impl IntoView {
    view! {
        <TabsListRoot
            variant=variant
            class=move || {
                cn!(
                    "group/tabs-list inline-flex w-fit items-center justify-center rounded-lg p-0.75 text-muted-foreground group-data-horizontal/tabs:h-8 group-data-vertical/tabs:h-fit group-data-vertical/tabs:flex-col data-[variant=line]:rounded-none",
                list_variant_class(variant),
                class.get()
                )
            }
        >
            {children()}
        </TabsListRoot>
    }
}

#[component]
pub fn TabsTrigger(
    #[prop(into)] value: String,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <TabsTriggerRoot
            value=value
            disabled=disabled
            class=move || {
                cn!(
                    "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-1.5 py-0.5 text-sm font-medium whitespace-nowrap text-foreground/60 transition-all group-data-vertical/tabs:w-full group-data-vertical/tabs:justify-start hover:text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 has-data-[icon=inline-end]:pr-1 has-data-[icon=inline-start]:pl-1 dark:text-muted-foreground dark:hover:text-foreground group-data-[variant=default]/tabs-list:data-active:shadow-sm group-data-[variant=line]/tabs-list:data-active:shadow-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
                "group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-active:bg-transparent dark:group-data-[variant=line]/tabs-list:data-active:border-transparent dark:group-data-[variant=line]/tabs-list:data-active:bg-transparent",
                "data-active:bg-primary/20 data-active:text-foreground dark:data-active:border-primary/30 dark:data-active:bg-primary/20 dark:data-active:text-foreground",
                "after:absolute after:bg-foreground after:opacity-0 after:transition-opacity group-data-horizontal/tabs:after:inset-x-0 group-data-horizontal/tabs:after:bottom-[-5px] group-data-horizontal/tabs:after:h-0.5 group-data-vertical/tabs:after:inset-y-0 group-data-vertical/tabs:after:-right-1 group-data-vertical/tabs:after:w-0.5 group-data-[variant=line]/tabs-list:data-active:after:opacity-100",
                class.get()
                )
            }
        >
            {children()}
        </TabsTriggerRoot>
    }
}

#[component]
pub fn TabsContent(
    #[prop(into)] value: String,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <TabsContentRoot
            value=value
            class=move || {
                cn!(
                    "flex-1 text-sm outline-none data-[state=inactive]:hidden",
                class.get()
                )
            }
        >
            {children()}
        </TabsContentRoot>
    }
}
