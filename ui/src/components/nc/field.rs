use leptos::prelude::*;
use nocomponents::cn;
use nocomponents::primitives::field::FieldDescriptionRoot;
use nocomponents::primitives::field::FieldErrorRoot;
use nocomponents::primitives::field::FieldLabelRoot;
use nocomponents::primitives::field::FieldLegendRoot;
use nocomponents::primitives::field::FieldRoot;
use nocomponents::primitives::field::FieldSetRoot;
use nocomponents::utils::types::Orientation;

#[component]
pub fn Field(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = Orientation::Vertical)] orientation: Orientation,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <FieldRoot
            orientation=orientation
            invalid=invalid
            disabled=disabled
            class=move || {
                cn!(
                    "group flex w-full gap-2 data-[orientation=vertical]:flex-col data-[orientation=horizontal]:flex-row data-[orientation=horizontal]:items-center data-[orientation=horizontal]:gap-3 data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-60",
                    class.get()
                )
            }
        >
            {children()}
        </FieldRoot>
    }
}

#[component]
pub fn FieldLabel(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <FieldLabelRoot class=move || {
            cn!(
                "flex items-center gap-2 text-sm leading-none font-medium select-none data-[invalid=true]:text-destructive data-[disabled=true]:opacity-50",
                class.get()
            )
        }>{children()}</FieldLabelRoot>
    }
}

#[component]
pub fn FieldDescription(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <FieldDescriptionRoot class=move || {
            cn!("text-sm leading-normal text-muted-foreground", class.get())
        }>{children()}</FieldDescriptionRoot>
    }
}

#[component]
pub fn FieldError(
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <FieldErrorRoot class=move || {
            cn!("text-sm leading-normal font-medium text-destructive", class.get())
        }>{children()}</FieldErrorRoot>
    }
}

#[component]
pub fn FieldGroup(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-slot="field-group" class=move || cn!("flex w-full flex-col gap-6", class.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn FieldSet(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <FieldSetRoot
            invalid=invalid
            disabled=disabled
            class=move || {
                cn!(
                    "group flex w-full min-w-0 flex-col gap-4 data-[disabled=true]:opacity-60",
                    class.get()
                )
            }
        >
            {children()}
        </FieldSetRoot>
    }
}

#[component]
pub fn FieldLegend(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <FieldLegendRoot class=move || {
            cn!(
                "mb-1 flex items-center gap-2 text-sm leading-none font-medium select-none data-[invalid=true]:text-destructive data-[disabled=true]:opacity-50",
                class.get()
            )
        }>{children()}</FieldLegendRoot>
    }
}
