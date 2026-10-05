use crate::components::nc::button::Button;
use crate::components::nc::button::ButtonSize;
use crate::components::nc::button::ButtonVariant;
use nocomponents::cn;
use nocomponents::icons::check::Check;
use nocomponents::icons::chevron::ChevronRight;
use nocomponents::primitives::dropdown_menu::DropdownMenuContentRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuItemRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuPortalRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuSubContentRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuSubRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuSubTriggerRoot;
use nocomponents::primitives::dropdown_menu::DropdownMenuTriggerRoot;
use nocomponents::primitives::floating::FloatingPortal;
use nocomponents::primitives::floating::TriggerRender;
use nocomponents::utils::types::Align;
use nocomponents::utils::types::Side;
use nocomponents::utils::types::SideOffset;
use leptos::either::Either;
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn DropdownMenu(children: ChildrenFn) -> impl IntoView {
    view! { <DropdownMenuRoot class="relative inline-block text-left">{children()}</DropdownMenuRoot> }
}

#[component]
pub fn DropdownMenuTrigger(
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] render: Option<Callback<TriggerRender, AnyView>>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let children = StoredValue::new(children);

    let styled = Callback::new(move |trigger: TriggerRender| {
        view! {
            <Button
                size=size
                variant=variant
                node_ref=trigger.node_ref
                on_click=trigger.on_click
                disabled=trigger.disabled
                class=class
            >
                {move || {
                    children
                        .with_value(|children| match children {
                            Some(child) => Either::Left(child()),
                            None => Either::Right(view! { "" }.into_any()),
                        })
                }}
            </Button>
        }
        .into_any()
    });

    view! { <DropdownMenuTriggerRoot disabled=disabled render=render.unwrap_or(styled) /> }
}

#[component]
pub fn DropdownMenuPortal(children: ChildrenFn) -> impl IntoView {
    view! { <DropdownMenuPortalRoot children=children /> }
}

#[component]
pub fn DropdownMenuContent(
    #[prop(optional)] side: Side,
    #[prop(optional)] align: Align,
    #[prop(optional)] side_offset: SideOffset,
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <DropdownMenuContentRoot
            side=side
            align=align
            side_offset=side_offset
            class=move || {
                cn!(
                    "z-50 min-w-32 overflow-hidden rounded-2xl border-2 bg-popover p-1.5 text-popover-foreground shadow-xl pointer-events-auto data-[state=closed]:pointer-events-none",
                    "data-[state=open]:animate-in data-[state=closed]:animate-out",
                    "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
                    "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
                    "data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
                    class.get()
                )
            }
        >
            {children()}
        </DropdownMenuContentRoot>
    }
}

#[component]
pub fn DropdownMenuItem(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <DropdownMenuItemRoot
            disabled=disabled
            on_click=on_click
            class=move || {
                cn!(
                    "relative flex font-medium cursor-default select-none items-center rounded-xl px-2 py-1.5 text-sm outline-none transition-colors focus:bg-accent focus:text-accent-foreground hover:bg-accent hover:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
                    class.get()
                )
            }
        >
            {children()}
        </DropdownMenuItemRoot>
    }
}

/// A menu item with a checkmark. Clicking it (or Enter/Space, which the menu
/// turns into a click) calls `on_change` with the new state and leaves the
/// menu open, so several can be ticked in a row. nocomponents has no
/// primitive for this: `DropdownMenuItemRoot` always closes the menu.
#[component]
pub fn DropdownMenuCheckboxItem(
    #[prop(into)] checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        // `data-roving-item` lets the menu's arrow keys and Enter/Space reach it.
        <div
            data-slot="dropdown-menu-checkbox-item"
            role="menuitemcheckbox"
            tabindex="-1"
            data-roving-item=""
            aria-checked=move || checked.get().to_string()
            data-state=move || if checked.get() { "checked" } else { "unchecked" }
            data-disabled=move || disabled.get().then_some("true")
            aria-disabled=move || disabled.get().then_some("true")
            on:click=move |_| {
                if !disabled.get_untracked() {
                    on_change.run(!checked.get_untracked());
                }
            }
            class=move || {
                cn!(
                    "relative flex font-medium cursor-default select-none items-center rounded-xl py-1.5 pr-2 pl-8 text-sm outline-none transition-colors focus:bg-accent focus:text-accent-foreground hover:bg-accent hover:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
                    class.get()
                )
            }
        >
            <span class="absolute left-2 flex size-4 items-center justify-center">
                <Show when=move || checked.get()>
                    <Check />
                </Show>
            </span>
            {children()}
        </div>
    }
}

#[component]
pub fn DropdownMenuLabel(children: Children) -> impl IntoView {
    view! { <div class="px-2 py-1.5 text-xs font-semibold text-muted-foreground">{children()}</div> }
}

#[component]
pub fn DropdownMenuSeparator() -> impl IntoView {
    view! { <div class="-mx-1 my-1 h-px bg-muted"></div> }
}

#[component]
pub fn DropdownMenuSub(children: ChildrenFn) -> impl IntoView {
    view! { <DropdownMenuSubRoot>{children()}</DropdownMenuSubRoot> }
}

#[component]
pub fn DropdownMenuSubTrigger(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <DropdownMenuSubTriggerRoot class=move || {
            cn!(
                "group flex w-full cursor-default select-none font-medium justify-between items-center rounded-xl px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground data-[state=open]:bg-accent data-[state=open]:text-accent-foreground",
                class.get()
            )
        }>{children()} <ChevronRight class="ml-auto" /></DropdownMenuSubTriggerRoot>
    }
}

#[component]
pub fn DropdownMenuSubContent(
    #[prop(optional)] side_offset: SideOffset,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let ctx = nocomponents::primitives::dropdown_menu::use_dropdown();
    let children = StoredValue::new(children);

    view! {
        <FloatingPortal>
            <Show when=move || ctx.is_mounted.get()>
                <DropdownMenuSubContentRoot
                    side_offset=side_offset
                    class=move || {
                        cn!(
                            "z-50 min-w-32 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-lg pointer-events-auto data-[state=closed]:pointer-events-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
                            class.get()
                        )
                    }
                >
                    {children.with_value(|c| c())}
                </DropdownMenuSubContentRoot>
            </Show>
        </FloatingPortal>
    }
}
