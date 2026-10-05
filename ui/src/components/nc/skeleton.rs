use leptos::prelude::*;
use nocomponents::cn;

#[component]
pub fn Skeleton(#[prop(optional, into)] class: Signal<String>) -> impl IntoView {
    view! { <div data-slot="skeleton" class=move || cn!("animate-pulse bg-muted", class.get()) /> }
}
