use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::switch::SwitchRoot;

#[component]
pub fn Switch(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(into)] checked: RwSignal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] id: Signal<String>,
) -> impl IntoView {
    view! {
        <SwitchRoot
            checked=checked
            disabled=disabled
            id=id
            class=move || {
                cn!(
                    "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
                    class.get()
                )
            }
        >
            <span
                data-state=move || if checked.get() { "checked" } else { "unchecked" }
                class="pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
            />
        </SwitchRoot>
    }
}
