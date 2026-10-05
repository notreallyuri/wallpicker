use nocomponents::cn;
use nocomponents::primitives::separator::SeparatorRoot;
use nocomponents::utils::types::Orientation;
use leptos::prelude::*;

#[component]
pub fn Separator(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional)] orientation: Orientation,
    #[prop(default = true)] decorative: bool,
) -> impl IntoView {
    view! {
        <SeparatorRoot
            orientation=orientation
            decorative=decorative
            class=move || {
                cn!(
                    "shrink-0 bg-border data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",
                    class.get()
                )
            }
        />
    }
}
