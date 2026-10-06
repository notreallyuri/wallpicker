use crate::components::nc::button::Button;
use crate::components::nc::button::ButtonSize;
use crate::components::nc::button::ButtonVariant;
use nocomponents::cn;
use nocomponents::icons::x::X;
use nocomponents::primitives::dialog::DialogContentRoot;
use nocomponents::primitives::dialog::DialogPart;
use nocomponents::primitives::dialog::DialogPortalRoot;
use nocomponents::primitives::dialog::DialogRoot;
use nocomponents::primitives::dialog::use_dialog;
use nocomponents::primitives::dialog::use_dialog_label_id;
use leptos::either::Either;
use leptos::ev;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;

#[component]
pub fn Dialog(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
    view! { <DialogRoot open=open>{children()}</DialogRoot> }
}

#[component]
pub fn DialogTrigger(
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] render: Option<Callback<Callback<ev::MouseEvent>, AnyView>>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let ctx = use_dialog();
    let on_click = Callback::new(move |_: ev::MouseEvent| ctx.toggle());

    match render {
        Some(render_fn) => Either::Left(render_fn.run(on_click)),
        None => Either::Right(view! {
            <Button
                size=size
                variant=variant
                class=class
                disabled=disabled
                on:click=move |e| on_click.run(e)
            >
                {
                    let children = children.clone();
                    move || match &children {
                        Some(child) => Either::Left(child()),
                        None => Either::Right(view! { "" }.into_any()),
                    }
                }
            </Button>
        }),
    }
}

#[component]
pub fn DialogPortal(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = true)] show_close: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let context = use_dialog();
    let stored_children = StoredValue::new(children);

    // wallpicker: move focus out as soon as the dialog starts closing, while
    // its content is still mounted (it unmounts after the exit animation).
    // A control that still has focus when it's removed gets a `focusout`
    // after its `Field` has been disposed, and the field's focusout handler
    // then panics writing to its `touched` signal. Seen with Esc while a
    // select inside a Field had focus.
    Effect::new(move |_| {
        if context.is_open.get() {
            return;
        }
        let focused_inside = document()
            .active_element()
            .filter(|el| el.closest("[data-slot=dialog-portal]").ok().flatten().is_some());
        if let Some(el) = focused_inside.and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = el.blur();
        }
    });

    view! {
        <DialogPortalRoot>
            <div data-slot="dialog-portal">
                <div
                    data-state=move || if context.is_open.get() { "open" } else { "closed" }
                    class="fixed inset-0 isolate z-50 bg-black/10 duration-100 supports-backdrop-filter:backdrop-blur-xs data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:pointer-events-none"
                    on:click=move |_| context.close()
                />

                <DialogContentRoot class=move || {
                    cn!(
                        "fixed top-1/2 left-1/2 z-50 grid w-full grid-cols-1 max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-4 rounded-3xl border-2 border-primary/15 bg-popover p-5 text-sm text-popover-foreground shadow-2xl ring-0 duration-200 ease-bounce outline-none sm:max-w-sm data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-90 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=closed]:pointer-events-none",
                            class.get()
                    )
                }>
                    {stored_children.with_value(|c| c())}
                    {show_close
                        .then(|| {
                            view! {
                                <button
                                    on:click=move |_| context.close()
                                    class="absolute right-4 top-4 rounded-full p-1 opacity-70 hover:bg-primary/12 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground"
                                >
                                    <X />
                                    <span class="sr-only">"Close"</span>
                                </button>
                            }
                        })}
                </DialogContentRoot>
            </div>
        </DialogPortalRoot>
    }
}

#[component]
pub fn DialogHeader(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=move || {
            cn!("flex flex-col space-y-1.5 text-center sm:text-left", class.get())
        }>{children()}</div>
    }
}

#[component]
pub fn DialogFooter(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=move || {
            cn!("flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2", class.get())
        }>{children()}</div>
    }
}

#[component]
pub fn DialogTitle(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    let id = use_dialog_label_id(DialogPart::Title);

    view! {
        <div
            id=id
            class=move || cn!("text-lg font-semibold leading-none tracking-tight", class.get())
        >
            {children()}
        </div>
    }
}

#[component]
pub fn DialogDescription(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    let id = use_dialog_label_id(DialogPart::Description);

    view! {
        <div id=id class=move || cn!("text-sm text-muted-foreground", class.get())>
            {children()}
        </div>
    }
}
