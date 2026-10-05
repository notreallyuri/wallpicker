use crate::components::nc::toggle::TOGGLE_BASE;
use crate::components::nc::toggle::ToggleSize;
use crate::components::nc::toggle::ToggleVariant;
use leptos::context::Provider;
use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::toggle_group::ToggleGroupItemRoot;
use nocomponents::primitives::toggle_group::ToggleGroupRoot;
use nocomponents::primitives::toggle_group::ToggleGroupType;
use nocomponents::utils::types::AsClass;
use nocomponents::utils::types::Orientation;

#[derive(Copy, Clone, Default)]
struct ToggleGroupStyle {
    variant: ToggleVariant,
    size: ToggleSize,
}

#[component]
pub fn ToggleGroup(
    #[prop(optional)] kind: ToggleGroupType,
    #[prop(optional)] orientation: Orientation,
    #[prop(optional)] variant: ToggleVariant,
    #[prop(optional)] size: ToggleSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = None, into)] value: Option<RwSignal<Vec<String>>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Provider value=ToggleGroupStyle { variant, size }>
            <ToggleGroupRoot
                kind=kind
                orientation=orientation
                disabled=disabled
                value=value
                class=move || {
                    cn!(
                        "group/toggle-group flex w-fit items-center rounded-lg data-[orientation=vertical]:flex-col data-[orientation=vertical]:items-stretch",
                        class.get()
                    )
                }
            >
                {children()}
            </ToggleGroupRoot>
        </Provider>
    }
}

#[component]
pub fn ToggleGroupItem(
    #[prop(into)] value: String,
    #[prop(optional)] variant: Option<ToggleVariant>,
    #[prop(optional)] size: Option<ToggleSize>,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let inherited = use_context::<ToggleGroupStyle>().unwrap_or_default();
    let variant = variant.unwrap_or(inherited.variant);
    let size = size.unwrap_or(inherited.size);

    view! {
        <ToggleGroupItemRoot
            value=value
            disabled=disabled
            class=move || {
                cn!(
                    TOGGLE_BASE,
                    variant.as_class(),
                    size.as_class(),
                    "rounded-none first:rounded-l-lg last:rounded-r-lg group-data-[orientation=vertical]/toggle-group:first:rounded-t-lg group-data-[orientation=vertical]/toggle-group:last:rounded-b-lg group-data-[orientation=vertical]/toggle-group:first:rounded-r-lg group-data-[orientation=vertical]/toggle-group:last:rounded-l-lg",
                    class.get()
                )
            }
        >
            {children.map(|c| c())}
        </ToggleGroupItemRoot>
    }
}
