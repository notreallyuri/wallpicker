use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::icons::check::Check;
use nocomponents::primitives::checkbox::CheckboxRoot;

#[component]
pub fn Checkbox(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(into)] checked: RwSignal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] id: Signal<String>,
) -> impl IntoView {
    view! {
        <CheckboxRoot
            id=id
            disabled=disabled
            checked=checked
            class=move || {
                cn!(
                    "peer size-5 shrink-0 rounded-md border-2 border-primary ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground",
                    class.get()
                )
            }
        >
            <div class=move || {
                cn!(
                    "flex items-center justify-center text-primary-foreground",
                    if checked.get() { "visible" } else { "hidden" }
                )
            }>
                <Check />
            </div>
        </CheckboxRoot>
    }
}
