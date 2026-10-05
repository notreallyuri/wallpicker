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
use nocomponents::utils::types::AsClass;
use nocomponents::utils::types::Side;
use leptos::either::Either;
use leptos::ev;
use leptos::prelude::*;

struct SheetSide(Side);

impl AsClass for SheetSide {
    fn as_class(&self) -> &'static str {
        match self.0 {
            Side::Right => {
                "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm data-[state=open]:slide-in-from-right data-[state=closed]:slide-out-to-right"
            }
            Side::Left => {
                "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm data-[state=open]:slide-in-from-left data-[state=closed]:slide-out-to-left"
            }
            Side::Top => {
                "inset-x-0 top-0 h-auto border-b data-[state=open]:slide-in-from-top data-[state=closed]:slide-out-to-top"
            }
            Side::Bottom => {
                "inset-x-0 bottom-0 h-auto border-t data-[state=open]:slide-in-from-bottom data-[state=closed]:slide-out-to-bottom"
            }
        }
    }
}

#[component]
pub fn Sheet(#[prop(optional)] open: Option<RwSignal<bool>>, children: Children) -> impl IntoView {
    view! { <DialogRoot open=open>{children()}</DialogRoot> }
}

#[component]
pub fn SheetTrigger(
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
pub fn SheetContent(
    #[prop(default = Side::Right)] side: Side,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let context = use_dialog();
    let stored_children = StoredValue::new(children);
    let side_class = SheetSide(side).as_class();

    view! {
        <DialogPortalRoot>
            <div data-slot="sheet-portal">
                <div
                    data-state=move || if context.is_open.get() { "open" } else { "closed" }
                    class="fixed inset-0 isolate z-50 bg-black/10 duration-100 supports-backdrop-filter:backdrop-blur-xs data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:pointer-events-none"
                    on:click=move |_| context.close()
                />

                <DialogContentRoot class=move || {
                    cn!(
                        "fixed z-50 flex flex-col gap-4 bg-popover p-4 text-sm text-popover-foreground shadow-lg duration-200 outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:pointer-events-none",
                        side_class,
                        class.get()
                    )
                }>
                    {stored_children.with_value(|children| children())}
                    <button
                        on:click=move |_| context.close()
                        class="absolute top-4 right-4 rounded-sm opacity-70 transition-opacity outline-none hover:opacity-100 focus-visible:ring-3 focus-visible:ring-ring/50"
                    >
                        <X />
                        <span class="sr-only">"Close"</span>
                    </button>
                </DialogContentRoot>
            </div>
        </DialogPortalRoot>
    }
}

#[component]
pub fn SheetHeader(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=move || {
            cn!("flex flex-col gap-1.5 pr-8", class.get())
        }>{children()}</div>
    }
}

#[component]
pub fn SheetTitle(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    let id = use_dialog_label_id(DialogPart::Title);

    view! {
        <div id=id class=move || cn!("text-base font-semibold tracking-tight", class.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn SheetDescription(
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

#[component]
pub fn SheetFooter(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=move || {
            cn!("mt-auto flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", class.get())
        }>{children()}</div>
    }
}

#[component]
pub fn SheetClose(
    #[prop(optional)] size: ButtonSize,
    #[prop(default = ButtonVariant::Outline)] variant: ButtonVariant,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let ctx = use_dialog();
    let stored_children = StoredValue::new(children);

    view! {
        <Button size=size variant=variant class=class on:click=move |_| ctx.close()>
            {move || stored_children.with_value(|children| children())}
        </Button>
    }
}
