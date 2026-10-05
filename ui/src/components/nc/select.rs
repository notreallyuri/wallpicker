use nocomponents::cn;
use nocomponents::icons::check::Check;
use nocomponents::icons::chevron::ChevronDown;
use nocomponents::primitives::select::SelectContentRoot;
use nocomponents::primitives::select::SelectItemRoot;
use nocomponents::primitives::select::SelectPortalRoot;
use nocomponents::primitives::select::SelectRoot;
use nocomponents::primitives::select::SelectTriggerRoot;
use nocomponents::primitives::select::use_select;
use leptos::prelude::*;

#[component]
pub fn Select(
    #[prop(optional, into)] value: RwSignal<Option<String>>,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <SelectRoot value=value class=move || cn!("relative w-full", class.get())>
            {children()}
        </SelectRoot>
    }
}

#[component]
pub fn SelectTrigger(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <SelectTriggerRoot class=move || {
            cn!(
                "flex h-8 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
                "aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40",
                class.get()
            )
        }>{children()} <ChevronDown class="text-muted-foreground" /></SelectTriggerRoot>
    }
}

#[component]
pub fn SelectValue(#[prop(into)] placeholder: String) -> impl IntoView {
    let ctx = use_select();

    view! {
        <span class="truncate">
            {move || ctx.value.get().unwrap_or_else(|| placeholder.clone())}
        </span>
    }
}

#[component]
pub fn SelectPortal(children: ChildrenFn) -> impl IntoView {
    view! { <SelectPortalRoot children=children /> }
}

#[component]
pub fn SelectContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <SelectContentRoot class=move || {
            cn!(
                "z-50 overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md pointer-events-auto data-[state=closed]:pointer-events-none",
                "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2",
                class.get()
            )
        }>
            <div class="p-1 w-full max-h-96 overflow-y-auto">{children()}</div>
        </SelectContentRoot>
    }
}

#[component]
pub fn SelectItem(
    #[prop(into)] value: String,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <SelectItemRoot
            value=value
            class=move || {
                cn!(
                    "group relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none hover:bg-accent hover:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
                    class.get()
                )
            }
        >
            <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
                <Check class="hidden group-data-[state=checked]:block" />
            </span>
            {children()}
        </SelectItemRoot>
    }
}
