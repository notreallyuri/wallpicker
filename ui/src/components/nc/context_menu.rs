use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::context_menu::ContextMenuContentRoot;
use nocomponents::primitives::context_menu::ContextMenuRoot;
use nocomponents::primitives::context_menu::ContextMenuTriggerRoot;
use nocomponents::utils::types::Align;
use nocomponents::utils::types::Side;
use nocomponents::utils::types::SideOffset;

pub use crate::components::nc::dropdown_menu::DropdownMenuCheckboxItem as ContextMenuCheckboxItem;
pub use crate::components::nc::dropdown_menu::DropdownMenuItem as ContextMenuItem;
pub use crate::components::nc::dropdown_menu::DropdownMenuLabel as ContextMenuLabel;
pub use crate::components::nc::dropdown_menu::DropdownMenuPortal as ContextMenuPortal;
pub use crate::components::nc::dropdown_menu::DropdownMenuSeparator as ContextMenuSeparator;
pub use crate::components::nc::dropdown_menu::DropdownMenuSub as ContextMenuSub;
pub use crate::components::nc::dropdown_menu::DropdownMenuSubContent as ContextMenuSubContent;
pub use crate::components::nc::dropdown_menu::DropdownMenuSubTrigger as ContextMenuSubTrigger;

#[component]
pub fn ContextMenu(children: ChildrenFn) -> impl IntoView {
    view! { <ContextMenuRoot>{children()}</ContextMenuRoot> }
}

#[component]
pub fn ContextMenuTrigger(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <ContextMenuTriggerRoot
            disabled=disabled
            class=move || cn!("select-none", class.get())
        >
            {children()}
        </ContextMenuTriggerRoot>
    }
}

#[component]
pub fn ContextMenuContent(
    #[prop(default = Side::Bottom)] side: Side,
    #[prop(default = Align::Start)] align: Align,
    #[prop(default = SideOffset(0.0))] side_offset: SideOffset,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    // Mounted only while open (and through the exit animation), like the
    // dropdown's content. Left mounted, a closed menu inside a hidden tab
    // panel replays its exit animation, a visible flash, whenever the panel
    // is shown again.
    let ctx = nocomponents::primitives::context_menu::use_context_menu();
    let children = StoredValue::new(children);

    view! {
        <Show when=move || ctx.is_mounted.get()>
            <ContextMenuContentRoot
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
                {children.with_value(|c| c())}
            </ContextMenuContentRoot>
        </Show>
    }
}
