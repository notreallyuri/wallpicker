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
                    "peer inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent shadow-inner transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
                    class.get()
                )
            }
        >
            <span
                data-state=move || if checked.get() { "checked" } else { "unchecked" }
                class="pointer-events-none block h-6 w-6 rounded-full bg-white shadow-[0_2px_0_0_rgb(0_0_0/0.18)] ring-0 transition-transform duration-300 ease-bounce data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
            />
        </SwitchRoot>
    }
}
