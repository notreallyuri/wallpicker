use leptos::ev;
use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::native_select::NativeSelectRoot;

#[derive(Default, Clone, Copy)]
pub enum NativeSelectSize {
    Sm,
    #[default]
    Default,
}

impl NativeSelectSize {
    fn as_class(&self) -> &'static str {
        match self {
            NativeSelectSize::Sm => "h-7 text-[0.8rem]",
            NativeSelectSize::Default => "h-8 text-sm",
        }
    }
}

#[component]
pub fn NativeSelect(
    #[prop(optional)] size: NativeSelectSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = None, into)] model: Option<RwSignal<String>>,
    #[prop(optional, into)] value: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] id: Signal<String>,
    #[prop(default = None, into)] on_blur: Option<Callback<ev::FocusEvent>>,
    children: Children,
) -> impl IntoView {
    let size_class = size.as_class();

    view! {
        <NativeSelectRoot
            id=id
            model=model
            value=value
            disabled=disabled
            on_blur=on_blur
            class=move || {
                cn!(
                    "w-full appearance-none rounded-lg border border-input bg-transparent py-1 pr-8 pl-2.5 transition-colors outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:bg-input/30 dark:disabled:bg-input/80 [&>optgroup]:bg-popover [&>option]:bg-popover",
                    "bg-no-repeat [background-position:right_0.625rem_center] [background-size:1rem_1rem]",
                    "[background-image:url(data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20viewBox=%220%200%2024%2024%22%20fill=%22none%22%20stroke=%22%23737373%22%20stroke-width=%222%22%20stroke-linecap=%22square%22%20stroke-linejoin=%22miter%22%3E%3Cpath%20d=%22m6%209%206%206%206-6%22/%3E%3C/svg%3E)]",
                    "dark:[background-image:url(data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20viewBox=%220%200%2024%2024%22%20fill=%22none%22%20stroke=%22%23a3a3a3%22%20stroke-width=%222%22%20stroke-linecap=%22square%22%20stroke-linejoin=%22miter%22%3E%3Cpath%20d=%22m6%209%206%206%206-6%22/%3E%3C/svg%3E)]",
                    size_class,
                    class.get()
                )
            }
        >
            {children()}
        </NativeSelectRoot>
    }
}
