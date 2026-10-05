use crate::components::nc::input::Input;
use crate::components::nc::separator::Separator;
use crate::components::nc::sheet::Sheet;
use crate::components::nc::sheet::SheetContent;
use crate::components::nc::sheet::SheetDescription;
use crate::components::nc::sheet::SheetTitle;
use crate::components::nc::skeleton::Skeleton;
use crate::components::nc::tooltip::Tooltip;
use crate::components::nc::tooltip::TooltipContent;
use crate::components::nc::tooltip::TooltipTrigger;
use nocomponents::cn;
use nocomponents::icons::chevron::ChevronRight;
use nocomponents::icons::panel::PanelLeft;
use nocomponents::primitives::sidebar::SidebarCollapsible;
use nocomponents::primitives::sidebar::SidebarMenuButtonRoot;
use nocomponents::primitives::sidebar::SidebarProviderRoot;
use nocomponents::primitives::sidebar::SidebarRailRoot;
use nocomponents::primitives::sidebar::SidebarRoot;
use nocomponents::primitives::sidebar::SidebarTriggerRoot;
use nocomponents::primitives::sidebar::use_sidebar;
use nocomponents::primitives::tree::TreeGroupRoot;
use nocomponents::primitives::tree::TreeItemRoot;
use nocomponents::primitives::tree::TreeRoot;
use nocomponents::primitives::tree::TreeRowRoot;
use nocomponents::primitives::tree::use_tree_item;
use nocomponents::utils::types::AsClass;
use nocomponents::utils::types::Side;
use nocomponents::utils::types::StyledRender;
use leptos::either::Either;
use leptos::ev;
use leptos::prelude::*;
use nocomponents::deps::leptos_node_ref::AnyNodeRef;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum SidebarVariant {
    #[default]
    Sidebar,
    Floating,
    Inset,
}

impl SidebarVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarVariant::Sidebar => "sidebar",
            SidebarVariant::Floating => "floating",
            SidebarVariant::Inset => "inset",
        }
    }

    fn is_inset(&self) -> bool {
        matches!(self, SidebarVariant::Floating | SidebarVariant::Inset)
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum SidebarMenuSize {
    #[default]
    Default,
    Sm,
    Lg,
}

impl SidebarMenuSize {
    fn as_str(&self) -> &'static str {
        match self {
            SidebarMenuSize::Default => "default",
            SidebarMenuSize::Sm => "sm",
            SidebarMenuSize::Lg => "lg",
        }
    }
}

impl AsClass for SidebarMenuSize {
    fn as_class(&self) -> &'static str {
        match self {
            SidebarMenuSize::Default => "h-8 text-sm",
            SidebarMenuSize::Sm => "h-7 text-xs",
            SidebarMenuSize::Lg => "h-12 text-sm group-data-[collapsible=icon]:!p-0",
        }
    }
}

#[component]
pub fn SidebarProvider(
    #[prop(default = Side::Left)] side: Side,
    #[prop(optional)] collapsible: SidebarCollapsible,
    #[prop(default = None, into)] open: Option<RwSignal<bool>>,
    #[prop(default = true)] default_open: bool,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] style: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <SidebarProviderRoot
            side=side
            collapsible=collapsible
            open=open
            default_open=default_open
            style=style
            class=move || {
                cn!(
                    "group/sidebar-wrapper flex min-h-svh w-full has-data-[variant=inset]:bg-sidebar",
                    class.get(),
                )
            }
        >
            {children()}
        </SidebarProviderRoot>
    }
}

#[component]
pub fn Sidebar(
    #[prop(optional)] variant: SidebarVariant,
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let ctx = use_sidebar();
    let stored_children = StoredValue::new(children);

    let panel = move || {
        view! {
            <div
                data-slot="sidebar-inner"
                class="flex h-full w-full flex-col bg-sidebar group-data-[variant=floating]:rounded-lg group-data-[variant=floating]:border group-data-[variant=floating]:border-sidebar-border group-data-[variant=floating]:shadow-sm"
            >
                {stored_children.with_value(|children| children())}
            </div>
        }
    };

    let gap_class = if variant.is_inset() {
        "group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+1rem)]"
    } else {
        "group-data-[collapsible=icon]:w-[var(--sidebar-width-icon)]"
    };

    let fixed_class = if variant.is_inset() {
        "p-2 group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+1rem+2px)]"
    } else {
        "group-data-[collapsible=icon]:w-[var(--sidebar-width-icon)] group-data-[side=left]:border-r group-data-[side=right]:border-l"
    };

    let edge_class = match ctx.side {
        Side::Right => {
            "right-0 group-data-[collapsible=offcanvas]:right-[calc(var(--sidebar-width)*-1)]"
        }
        _ => "left-0 group-data-[collapsible=offcanvas]:left-[calc(var(--sidebar-width)*-1)]",
    };

    move || {
        if ctx.is_mobile.get() {
            Either::Left(view! {
                <Sheet open=ctx.open_mobile>
                    <SheetContent
                        side=ctx.side
                        class="w-[var(--sidebar-width-mobile,18rem)] bg-sidebar p-0 text-sidebar-foreground [&>button]:hidden"
                    >
                        <SheetTitle class="sr-only">"Sidebar"</SheetTitle>
                        <SheetDescription class="sr-only">
                            "Navigation for this application."
                        </SheetDescription>
                        <div class="flex h-full w-full flex-col">{panel()}</div>
                    </SheetContent>
                </Sheet>
            })
        } else {
            Either::Right(view! {
                <SidebarRoot
                    variant=variant.as_str()
                    class=move || {
                        cn!(
                            "group peer hidden text-sidebar-foreground md:block",
                            "data-[side=right]:order-last",
                            class.get(),
                        )
                    }
                >
                    <div class=cn!(
                        "relative w-[var(--sidebar-width)] bg-transparent transition-[width] duration-200 ease-linear",
                        "group-data-[collapsible=offcanvas]:w-0 group-data-[side=right]:rotate-180",
                        gap_class,
                    ) />
                    <div class=cn!(
                        "fixed inset-y-0 z-10 hidden w-[var(--sidebar-width)] transition-[left,right,width] duration-200 ease-linear md:flex",
                        edge_class,
                        fixed_class,
                    )>{panel()}</div>
                </SidebarRoot>
            })
        }
    }
}

#[component]
pub fn SidebarInset(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <main
            data-slot="sidebar-inset"
            class=move || {
                cn!(
                    "relative flex w-full flex-1 flex-col bg-background",
                    "md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow-sm",
                    "md:peer-data-[variant=inset]:peer-data-[state=collapsed]:ml-2",
                    class.get(),
                )
            }
        >
            {children()}
        </main>
    }
}

#[component]
pub fn SidebarTrigger(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <SidebarTriggerRoot
            disabled=disabled
            class=move || {
                cn!(
                    "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50",
                    class.get(),
                )
            }
        >
            {match children {
                Some(children) => Either::Left(children()),
                None => {
                    Either::Right(
                        view! {
                            <PanelLeft />
                            <span class="sr-only">"Toggle sidebar"</span>
                        },
                    )
                }
            }}
        </SidebarTriggerRoot>
    }
}

#[component]
pub fn SidebarRail(#[prop(optional, into)] class: Signal<String>) -> impl IntoView {
    view! {
        <SidebarRailRoot class=move || {
            cn!(
                "absolute inset-y-0 z-20 hidden w-4 -translate-x-1/2 transition-all ease-linear after:absolute after:inset-y-0 after:left-1/2 after:w-[2px] hover:after:bg-sidebar-border sm:flex",
                "group-data-[side=left]:-right-4 group-data-[side=right]:left-0",
                "group-data-[side=left]:cursor-w-resize group-data-[side=right]:cursor-e-resize",
                "group-data-[collapsible=offcanvas]:translate-x-0 group-data-[collapsible=offcanvas]:after:left-full group-data-[collapsible=offcanvas]:hover:bg-sidebar",
                "group-data-[collapsible=none]:hidden",
                class.get(),
            )
        } />
    }
}

#[component]
pub fn SidebarHeader(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-slot="sidebar-header" class=move || cn!("flex flex-col gap-2 p-2", class.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarFooter(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-slot="sidebar-footer" class=move || cn!("flex flex-col gap-2 p-2", class.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="sidebar-content"
            class=move || {
                cn!(
                    "flex min-h-0 flex-1 flex-col gap-2 overflow-auto group-data-[collapsible=icon]:overflow-hidden",
                    class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarSeparator(#[prop(optional, into)] class: Signal<String>) -> impl IntoView {
    view! { <Separator class=move || cn!("mx-2 w-auto bg-sidebar-border", class.get()) /> }
}

#[component]
pub fn SidebarInput(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = None, into)] model: Option<RwSignal<String>>,
    #[prop(optional, into)] placeholder: Signal<String>,
) -> impl IntoView {
    view! {
        <Input
            model=model
            attr:placeholder=move || placeholder.get()
            class=move || cn!("h-8 w-full bg-background shadow-none", class.get())
        />
    }
}

#[component]
pub fn SidebarGroup(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="sidebar-group"
            class=move || cn!("relative flex w-full min-w-0 flex-col p-2", class.get())
        >
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarGroupLabel(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="sidebar-group-label"
            class=move || {
                cn!(
                    "flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium text-sidebar-foreground/70 outline-none ring-sidebar-ring transition-[margin,opacity] duration-200 ease-linear focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0",
                    "group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0",
                    class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarGroupAction(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type="button"
            data-slot="sidebar-group-action"
            class=move || {
                cn!(
                    "absolute top-3.5 right-3 flex aspect-square w-5 items-center justify-center rounded-md p-0 text-sidebar-foreground outline-none ring-sidebar-ring transition-transform hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0",
                    "after:absolute after:-inset-2 md:after:hidden",
                    "group-data-[collapsible=icon]:hidden",
                    class.get(),
                )
            }
        >
            {children()}
        </button>
    }
}

#[component]
pub fn SidebarGroupContent(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-slot="sidebar-group-content" class=move || cn!("w-full text-sm", class.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarMenu(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <ul
            data-slot="sidebar-menu"
            class=move || cn!("flex w-full min-w-0 flex-col gap-1", class.get())
        >
            {children()}
        </ul>
    }
}

#[component]
pub fn SidebarMenuItem(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <li
            data-slot="sidebar-menu-item"
            class=move || cn!("group/menu-item relative", class.get())
        >
            {children()}
        </li>
    }
}

#[component]
pub fn SidebarMenuButton(
    #[prop(optional, into)] active: Signal<bool>,
    #[prop(optional)] size: SidebarMenuSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = None, into)] tooltip: Option<Signal<String>>,
    #[prop(default = None, into)] render: Option<Callback<StyledRender, AnyView>>,
    #[prop(optional)] node_ref: Option<AnyNodeRef>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let ctx = use_sidebar();
    let size_class = size.as_class();
    let stored_children = StoredValue::new(children);
    let node_ref = node_ref.unwrap_or_default();

    let button = move || {
        let children = stored_children.get_value();
        view! {
            <SidebarMenuButtonRoot
                active=active
                size=size.as_str()
                disabled=disabled
                render=render
                node_ref=node_ref
                class=move || {
                    cn!(
                        "peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-left text-sm outline-none ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50",
                        "data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground",
                        "data-[state=open]:hover:bg-sidebar-accent data-[state=open]:hover:text-sidebar-accent-foreground",
                        "group-has-data-[slot=sidebar-menu-action]/menu-item:pr-8",
                        "group-data-[collapsible=icon]:!size-8 group-data-[collapsible=icon]:!p-2",
                        "[&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0",
                        size_class,
                        class.get(),
                    )
                }
            >
                {children.map(|children| children())}
            </SidebarMenuButtonRoot>
        }
    };

    move || match tooltip {
        Some(tooltip) if ctx.state() == "collapsed" && !ctx.is_mobile.get() => {
            Either::Left(view! {
                <Tooltip>
                    <TooltipTrigger>{button()}</TooltipTrigger>
                    <TooltipContent side=Side::Right>{move || tooltip.get()}</TooltipContent>
                </Tooltip>
            })
        }
        _ => Either::Right(button()),
    }
}

#[component]
pub fn SidebarMenuAction(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = false)] show_on_hover: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type="button"
            data-slot="sidebar-menu-action"
            class=move || {
                cn!(
                    "absolute top-1.5 right-1 flex aspect-square w-5 items-center justify-center rounded-md p-0 text-sidebar-foreground outline-none ring-sidebar-ring transition-transform hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 peer-hover/menu-button:text-sidebar-accent-foreground [&>svg]:size-4 [&>svg]:shrink-0",
                    "after:absolute after:-inset-2 md:after:hidden",
                    "peer-data-[size=sm]/menu-button:top-1 peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5",
                    "group-data-[collapsible=icon]:hidden",
                    if show_on_hover {
                        "opacity-0 focus-within:opacity-100 group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 peer-data-[active=true]/menu-button:opacity-100 md:opacity-0"
                    } else {
                        ""
                    },
                    class.get(),
                )
            }
        >
            {children()}
        </button>
    }
}

#[component]
pub fn SidebarMenuBadge(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-slot="sidebar-menu-badge"
            class=move || {
                cn!(
                    "pointer-events-none absolute right-1 flex h-5 min-w-5 items-center justify-center rounded-md px-1 text-xs font-medium tabular-nums text-sidebar-foreground select-none",
                    "peer-hover/menu-button:text-sidebar-accent-foreground peer-data-[active=true]/menu-button:text-sidebar-accent-foreground",
                    "peer-data-[size=sm]/menu-button:top-1 peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5",
                    "group-data-[collapsible=icon]:hidden",
                    class.get(),
                )
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn SidebarMenuSkeleton(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(default = true)] show_icon: bool,
    #[prop(default = 70)] width: u32,
) -> impl IntoView {
    view! {
        <div
            data-slot="sidebar-menu-skeleton"
            class=move || cn!("flex h-8 items-center gap-2 rounded-md px-2", class.get())
        >
            {show_icon.then(|| view! { <Skeleton class="size-4 rounded-md" /> })}
            <Skeleton
                class="h-4 max-w-[--skeleton-width] flex-1"
                attr:style=format!("--skeleton-width: {width}%")
            />
        </div>
    }
}

#[component]
pub fn SidebarTree(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] label: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <TreeRoot
            label=label
            class=move || {
                cn!(
                    "flex w-full min-w-0 flex-col gap-1", "group-data-[collapsible=icon]:hidden", class.get()
                )
            }
        >
            {children()}
        </TreeRoot>
    }
}

#[component]
pub fn SidebarTreeItem(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional)] default_open: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <TreeItemRoot
            default_open=default_open
            class=move || cn!("flex min-w-0 flex-col gap-1 outline-none", class.get())
        >
            {children()}
        </TreeItemRoot>
    }
}

#[component]
pub fn SidebarTreeRow(
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] active: Signal<bool>,
    #[prop(default = None, into)] on_select: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let item = use_tree_item();
    let has_group = move || item.is_some_and(|item| item.has_group.get());

    view! {
        <TreeRowRoot
            on_select=on_select
            class=move || {
                cn!(
                    "group/tree-row flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-left text-sm outline-none ring-sidebar-ring transition-[width,height,padding]",
                    "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground",
                    "data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground",
                    "[[data-slot=tree-item]:focus-visible>&]:ring-2",
                    "[&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0",
                    class.get(),
                )
            }
            attr:data-active=move || active.get().then_some("true")
        >
            <span class="flex size-4 shrink-0 items-center justify-center">
                <Show when=has_group>
                    <ChevronRight class="size-3.5 text-sidebar-foreground/70 transition-transform group-data-[state=open]/tree-row:rotate-90" />
                </Show>
            </span>
            <span class="truncate">{children()}</span>
        </TreeRowRoot>
    }
}

#[component]
pub fn SidebarTreeGroup(
    #[prop(optional, into)] class: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <TreeGroupRoot class=move || {
            cn!(
                "mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-l border-sidebar-border px-2.5 py-0.5",
                class.get(),
            )
        }>{children()}</TreeGroupRoot>
    }
}

#[component]
pub fn SidebarMenuSub(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <ul
            data-slot="sidebar-menu-sub"
            class=move || {
                cn!(
                    "mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-l border-sidebar-border px-2.5 py-0.5",
                    "group-data-[collapsible=icon]:hidden",
                    class.get(),
                )
            }
        >
            {children()}
        </ul>
    }
}

#[component]
pub fn SidebarMenuSubItem(
    #[prop(optional, into)] class: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <li
            data-slot="sidebar-menu-sub-item"
            class=move || cn!("group/menu-sub-item relative", class.get())
        >
            {children()}
        </li>
    }
}

#[component]
pub fn SidebarMenuSubButton(
    #[prop(optional, into)] active: Signal<bool>,
    #[prop(default = SidebarMenuSize::Default)] size: SidebarMenuSize,
    #[prop(optional, into)] class: Signal<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = None, into)] render: Option<Callback<StyledRender, AnyView>>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let size_class = match size {
        SidebarMenuSize::Sm => "text-xs",
        _ => "text-sm",
    };

    view! {
        <SidebarMenuButtonRoot
            active=active
            size=size.as_str()
            disabled=disabled
            render=render
            class=move || {
                cn!(
                    "flex h-7 min-w-0 -translate-x-px items-center gap-2 overflow-hidden rounded-md px-2 text-sidebar-foreground outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50",
                    "data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground",
                    "[&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 [&>svg]:text-sidebar-accent-foreground",
                    size_class,
                    class.get(),
                )
            }
        >
            {children.map(|children| children())}
        </SidebarMenuButtonRoot>
    }
}
