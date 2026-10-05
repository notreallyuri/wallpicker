use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::deps::leptos_node_ref::AnyNodeRef;
use nocomponents::primitives::textarea::TextareaRoot;

#[component]
pub fn Textarea(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = None, into)] model: Option<RwSignal<String>>,
    #[prop(optional, into)] value: Signal<String>,
    #[prop(optional, into)] id: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = false)] auto_resize: bool,
    #[prop(optional)] node_ref: Option<AnyNodeRef>,
) -> impl IntoView {
    let node_ref = node_ref.unwrap_or_default();

    view! {
        <TextareaRoot
            value=value
            model=model
            id=id
            disabled=disabled
            class=move || {
                cn!(
                    "flex min-h-16 w-full rounded-xl border-2 border-input bg-card/60 px-3 py-2 text-base transition-colors outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40",
                    class.get()
                )
            }
            auto_resize=auto_resize
            node_ref=node_ref
        />
    }
}
