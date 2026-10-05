use nocomponents::cn;
use nocomponents::primitives::tooltip::TooltipContentRoot;
use nocomponents::primitives::tooltip::TooltipPortalRoot;
use nocomponents::primitives::tooltip::TooltipRoot;
use nocomponents::primitives::tooltip::TooltipTriggerRoot;
use nocomponents::utils::types::Align;
use nocomponents::utils::types::AnchorRender;
use nocomponents::utils::types::Side;
use nocomponents::utils::types::SideOffset;
use leptos::prelude::*;

#[component]
pub fn Tooltip(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = 400, into)] delay: u64,
    #[prop(default = 100, into)] close_delay: u64,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <TooltipRoot
            delay=delay
            close_delay=close_delay
            class=move || cn!("inline-block", class.get())
        >
            {children()}
        </TooltipRoot>
    }
}

#[component]
pub fn TooltipTrigger(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = None, into)] render: Option<Callback<AnchorRender, AnyView>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <TooltipTriggerRoot
            disabled=disabled
            render=render
            class=move || cn!("inline-flex w-fit", class.get())
        >
            {children.map(|children| children())}
        </TooltipTriggerRoot>
    }
}

#[component]
pub fn TooltipContent(
    #[prop(default = Side::Top)] side: Side,
    #[prop(optional)] align: Align,
    #[prop(default = SideOffset(6.0))] side_offset: SideOffset,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let stored_children = StoredValue::new(children);

    view! {
        <TooltipPortalRoot>
            <TooltipContentRoot
                side=side
                align=align
                side_offset=side_offset
                class=move || {
                    cn!(
                        "w-fit rounded-md bg-foreground px-2 py-1 text-xs text-background animation-duration-100 fill-mode-forwards data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
                        class.get()
                    )
                }
            >
                {stored_children.with_value(|children| children())}
            </TooltipContentRoot>
        </TooltipPortalRoot>
    }
}
