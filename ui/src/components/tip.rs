//! `Tip`: nocomponents' tooltip around a control, for the ones that explain
//! something (icon-only buttons, terse labels). Not the browser's `title`,
//! which looks out of place.

use leptos::prelude::*;
use nocomponents::utils::types::Side;

use crate::components::nc::tooltip::{Tooltip, TooltipContent, TooltipTrigger};

#[component]
pub(crate) fn Tip(
    /// What it says; empty shows nothing.
    #[prop(into)]
    text: Signal<String>,
    /// For the wrapper: layout classes that belonged to the control (`ml-auto`).
    #[prop(optional, into)]
    class: String,
    #[prop(default = Side::Bottom)] side: Side,
    children: ChildrenFn,
) -> impl IntoView {
    let children = StoredValue::new(children);
    view! {
        <Tooltip class=class>
            <TooltipTrigger>{children.with_value(|c| c())}</TooltipTrigger>
            <Show when=move || text.with(|t| !t.is_empty())>
                <TooltipContent side=side class="max-w-72 whitespace-pre-line">{move || text.get()}</TooltipContent>
            </Show>
        </Tooltip>
    }
}
