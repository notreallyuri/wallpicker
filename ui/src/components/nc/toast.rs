use leptos::prelude::*;
pub use nocomponents::cn;
use nocomponents::deps::leptos_node_ref::AnyNodeRef;
pub use nocomponents::primitives::toast::ToastAction;
pub use nocomponents::primitives::toast::ToastContext;
pub use nocomponents::primitives::toast::ToastData;
pub use nocomponents::primitives::toast::ToastPosition;
pub use nocomponents::primitives::toast::ToastProviderRoot;
pub use nocomponents::primitives::toast::ToastRoot;
pub use nocomponents::primitives::toast::ToastSize;
pub use nocomponents::primitives::toast::ToastVariant;
pub use nocomponents::primitives::toast::use_toast;

fn variant_class(variant: ToastVariant) -> &'static str {
    match variant {
        ToastVariant::Default => "bg-popover text-foreground",
        ToastVariant::Destructive => {
            "border-destructive bg-destructive text-destructive-foreground"
        }
    }
}

const STACK_GAP: f64 = 8.0;
const STACK_SCALE_STEP: f64 = 0.05;
const STACK_VISIBLE_COUNT: usize = 3;
const EXPAND_GAP: f64 = 8.0;

#[component]
pub fn ToastProvider(
    #[prop(optional)] default_variant: Option<ToastVariant>,
    #[prop(optional)] default_size: Option<ToastSize>,
    #[prop(optional)] default_position: Option<ToastPosition>,
    children: Children,
) -> impl IntoView {
    view! {
        <ToastProviderRoot
            default_variant=default_variant
            default_size=default_size
            default_position=default_position
        >
            {children()}
            <Toaster />
        </ToastProviderRoot>
    }
}

#[component]
fn Toaster() -> impl IntoView {
    let ctx = use_toast();

    let regions: &'static [(ToastPosition, &'static str)] = &[
        (ToastPosition::TopLeft, "top-4 left-4"),
        (ToastPosition::TopCenter, "top-4 left-1/2 -translate-x-1/2"),
        (ToastPosition::TopRight, "top-4 right-4"),
        (ToastPosition::BottomLeft, "bottom-4 left-4"),
        (
            ToastPosition::BottomCenter,
            "bottom-4 left-1/2 -translate-x-1/2",
        ),
        (ToastPosition::BottomRight, "bottom-4 right-4"),
    ];

    view! {
        {regions
            .iter()
            .map(|(pos, placement)| {
                let pos = *pos;
                let is_hovered = RwSignal::new(false);
                let is_interacting = RwSignal::new(false);
                let is_focused = RwSignal::new(false);
                let expanded = Memo::new(move |_| {
                    is_hovered.get() || is_interacting.get() || is_focused.get()
                });

                view! {
                    <div
                        class=move || {
                            let has_toasts = ctx.toasts.get().iter().any(|t| t.position == pos);
                            if has_toasts {
                                format!(
                                    "fixed z-[100] w-full sm:w-[356px] pointer-events-none {placement}",
                                )
                            } else {
                                "hidden".to_string()
                            }
                        }
                        on:pointerenter=move |_| is_hovered.set(true)
                        on:pointerleave=move |_| is_hovered.set(false)
                        on:focusin=move |_| is_focused.set(true)
                        on:focusout=move |_| is_focused.set(false)
                    >
                        <div
                            class="w-full pointer-events-auto"
                            style=move || {
                                let toasts: Vec<ToastData> = ctx
                                    .toasts
                                    .get()
                                    .into_iter()
                                    .filter(|t| t.position == pos)
                                    .collect();
                                if toasts.is_empty() {
                                    return "position: relative;".to_string();
                                }
                                if expanded.get() {
                                    let mut v = toasts.clone();
                                    v.reverse();
                                    let total_height: f64 = v
                                        .iter()
                                        .enumerate()
                                        .map(|(i, t)| {
                                            let h = t.height.get();
                                            let h = if h > 0.0 { h } else { 64.0 };
                                            if i == 0 { h } else { h + EXPAND_GAP }
                                        })
                                        .sum();
                                    format!(
                                        "position: relative; height: {total_height:.1}px; \
                                         transition: height 400ms ease;",
                                    )
                                } else {
                                    let front_height = front_toast_height(&toasts, pos);
                                    let visible = toasts.len().min(STACK_VISIBLE_COUNT);
                                    let extra = (visible.saturating_sub(1)) as f64 * STACK_GAP;
                                    format!(
                                        "position: relative; height: {:.1}px; \
                                         transition: height 400ms ease;",
                                        front_height + extra,
                                    )
                                }
                            }
                        >
                            <ToastRegion
                                pos=pos
                                ctx=ctx
                                expanded=expanded
                                is_interacting=is_interacting
                            />
                        </div>
                    </div>
                }
            })
            .collect_view()}
    }
}

#[component]
fn ToastRegion(
    pos: ToastPosition,
    ctx: ToastContext,
    expanded: Memo<bool>,
    is_interacting: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <For
            each=move || {
                let mut toasts: Vec<ToastData> = ctx
                    .toasts
                    .get()
                    .into_iter()
                    .filter(|t| t.position == pos)
                    .collect();
                toasts.reverse();
                toasts
            }
            key=|t| t.id
            children=move |toast| {
                view! {
                    <ToastItem
                        toast=toast
                        expanded=expanded
                        is_interacting=is_interacting
                        pos=pos
                        ctx=ctx
                    />
                }
            }
        />
    }
}

#[component]
fn ToastItem(
    toast: ToastData,
    expanded: Memo<bool>,
    is_interacting: RwSignal<bool>,
    pos: ToastPosition,
    ctx: ToastContext,
) -> impl IntoView {
    let node_ref = AnyNodeRef::new();

    let index = Memo::new(move |_| {
        let mut toasts: Vec<ToastData> = ctx
            .toasts
            .get()
            .into_iter()
            .filter(|t| t.position == pos)
            .collect();
        toasts.reverse();
        toasts.iter().position(|t| t.id == toast.id).unwrap_or(0)
    });

    let total = Memo::new(move |_| {
        ctx.toasts
            .get()
            .iter()
            .filter(|t| t.position == pos)
            .count()
    });

    let remaining = RwSignal::new(toast.duration);
    let last_start = RwSignal::new(nocomponents::deps::js_sys::Date::now());
    let timeout_handle: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);

    Effect::new(move |_| {
        if expanded.get() {
            if let Some(handle) = timeout_handle.get_value() {
                handle.clear();
                timeout_handle.set_value(None);
            }
            let elapsed = nocomponents::deps::js_sys::Date::now() - last_start.get();
            remaining.update(|r| *r = (*r - elapsed).max(0.0));
        } else {
            last_start.set(nocomponents::deps::js_sys::Date::now());
            let r = remaining.get();

            if r > 0.0 {
                let id = toast.id;
                if let Ok(handle) = set_timeout_with_handle(
                    move || ctx.dismiss(id),
                    std::time::Duration::from_millis(r as u64),
                ) {
                    timeout_handle.set_value(Some(handle));
                }
            } else {
                ctx.dismiss(toast.id);
            }
        }
    });

    on_cleanup(move || {
        if let Some(handle) = timeout_handle.get_value() {
            handle.clear();
        }
    });

    Effect::new(move |_| {
        if let Some(el) = node_ref.get()
            && let Some(parent) = el.parent_element()
        {
            let h = parent.get_bounding_client_rect().height();
            if h > 0.0 {
                toast.height.set(h);
            }
        }
    });

    let is_top = matches!(
        pos,
        ToastPosition::TopLeft | ToastPosition::TopCenter | ToastPosition::TopRight
    );

    let exit_classes = match pos {
        ToastPosition::TopRight | ToastPosition::BottomRight => {
            "data-[state=closed]:translate-x-[120%]"
        }
        ToastPosition::TopLeft | ToastPosition::BottomLeft => {
            "data-[state=closed]:-translate-x-[120%]"
        }
        ToastPosition::TopCenter => "data-[state=closed]:-translate-y-[150%]",
        ToastPosition::BottomCenter => "data-[state=closed]:translate-y-[150%]",
    };

    view! {
        <ToastRoot
            toast=toast.clone()
            is_interacting=is_interacting
            class=move || {
                cn!(
                    "before:absolute before:-inset-y-4 before:inset-x-0 before:z-[-1] before:content-['']",
                    "absolute left-0 w-full flex items-center justify-between space-x-2 overflow-hidden rounded-md border shadow-lg",
                    if is_top { "top-0 origin-top" } else { "bottom-0 origin-bottom" },
                    "translate-x-[var(--drag-x,0px)] translate-y-[calc(var(--toast-y,0px)+var(--drag-y,0px))] scale-[var(--toast-scale,1)]",
                    "z-[var(--toast-z,100)] opacity-[var(--toast-opacity,1)]",
                    "transition-all duration-[400ms] ease-[cubic-bezier(0.32,0.72,0,1)]",
                    "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
                    "data-[state=closed]:opacity-0",
                    exit_classes,
                    "data-[dragging=false]:cursor-grab data-[dragging=true]:cursor-grabbing data-[dragging=true]:transition-none",
                    "text-popover-foreground touch-none select-none",
                    "py-3 px-4 text-sm data-[size=lg]:py-6 data-[size=lg]:px-8 data=[size=lg]:text-base",
                    variant_class(toast.variant),
                )
            }
            style=move || {
                if expanded.get() {
                    expanded_style(toast.id, pos, ctx)
                } else {
                    collapsed_style(index.get(), total.get(), pos)
                }
            }
        >
            <div node_ref=node_ref class="min-w-0 flex-1">
                <div class="grid gap-1">
                    <div class="text-sm font-medium">{toast.title.clone()}</div>
                    {toast
                        .description
                        .clone()
                        .map(|desc| {
                            view! {
                                <div class=move || {
                                    cn!(
                                        "text-xs font-medium", match toast.variant {
                                            ToastVariant::Default => "text-muted-foreground",
                                            ToastVariant::Destructive => "text-primary"
                                        }
                                    )
                                }>{desc}</div>
                            }
                        })}
                </div>
            </div>

            {toast
                .action
                .clone()
                .map(|action| {
                    let id = toast.id;
                    view! {
                        <button
                            type="button"
                            on:click=move |_| {
                                (action.on_press)();
                                ctx.dismiss(id);
                            }
                            class=move || {
                                cn!(
                                    "shrink-0 rounded-md px-2.5 py-1 text-xs font-medium ring-1 transition-colors focus-visible:outline-none focus-visible:ring-2",
                                    match toast.variant {
                                        ToastVariant::Default => {
                                            "ring-border hover:bg-accent hover:text-accent-foreground"
                                        }
                                        ToastVariant::Destructive => {
                                            "ring-destructive-foreground/40 hover:bg-destructive-foreground/10"
                                        }
                                    },
                                )
                            }
                        >
                            {action.label}
                        </button>
                    }
                })}
        </ToastRoot>
    }
}

fn collapsed_style(index: usize, _total: usize, pos: ToastPosition) -> String {
    let is_top = matches!(
        pos,
        ToastPosition::TopLeft | ToastPosition::TopCenter | ToastPosition::TopRight
    );
    let sign = if is_top { 1.0 } else { -1.0 };

    let offset = index as f64 * STACK_GAP;
    let scale = 1.0 - (index as f64 * STACK_SCALE_STEP).min(1.0);

    let opacity = if index >= STACK_VISIBLE_COUNT {
        0.0
    } else if index == 0 {
        1.0
    } else {
        (1.0 - index as f64 * 0.15).max(0.0)
    };
    let pointer_events = if index >= STACK_VISIBLE_COUNT {
        "none"
    } else {
        "auto"
    };

    format!(
        "--toast-y: {:.1}px; --toast-scale: {:.3}; --toast-z: {}; --toast-opacity: {:.3}; pointer-events: {};",
        offset * sign,
        scale,
        100usize.saturating_sub(index),
        opacity,
        pointer_events
    )
}

fn expanded_style(toast_id: usize, pos: ToastPosition, ctx: ToastContext) -> String {
    let is_top = matches!(
        pos,
        ToastPosition::TopLeft | ToastPosition::TopCenter | ToastPosition::TopRight
    );
    let sign = if is_top { 1.0 } else { -1.0 };

    let mut toasts: Vec<ToastData> = ctx
        .toasts
        .get()
        .into_iter()
        .filter(|t| t.position == pos)
        .collect();
    toasts.reverse();

    let Some(my_index) = toasts.iter().position(|t| t.id == toast_id) else {
        return String::new();
    };
    let offset_from_edge: f64 = toasts[..my_index]
        .iter()
        .map(|t| t.height.get() + EXPAND_GAP)
        .sum();

    format!(
        "--toast-y: {:.1}px; --toast-scale: 1; --toast-z: {}; --toast-opacity: 1; pointer-events: auto;",
        offset_from_edge * sign,
        100usize.saturating_sub(my_index)
    )
}

fn front_toast_height(toasts: &[ToastData], _pos: ToastPosition) -> f64 {
    toasts
        .last()
        .map(|t| {
            let h = t.height.get();
            if h > 0.0 { h } else { 64.0 }
        })
        .unwrap_or(64.0)
}
