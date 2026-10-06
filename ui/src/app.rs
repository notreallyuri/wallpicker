//! The wallpicker UI, built on nocomponents.
//!
//! The styled components live in `src/components/nc/` (installed by
//! `cargo nocli`); the wallpaper tiles are plain markup styled with Tailwind
//! against the same theme tokens, so everything follows the light/dark mode.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::primitives::tabs::use_tabs;
use nocomponents::utils::theme::ThemeProvider;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::monitor_bar::MonitorBar;
use crate::views::local::LocalView;
use crate::views::settings::SettingsDialog;
use crate::views::online::OnlineView;
use crate::views::online::collections::CollectionsView;
use crate::components::tip::Tip;

const TAB_LOCAL: &str = "local";
pub(crate) const TAB_ONLINE: &str = "online";
pub(crate) const TAB_COLLECTIONS: &str = "collections";

#[component]
pub fn App() -> impl IntoView {
    view! {
        <ThemeProvider>
            <ToastProvider>
                <Shell />
            </ToastProvider>
        </ThemeProvider>
    }
}

/// Everything below the providers, so it can pick up the toast context.
#[component]
fn Shell() -> impl IntoView {
    let ctx = Ctx::new(use_toast());
    provide_context(ctx);
    crate::shortcuts::install();
    // After the first render, when the tabs' headers exist.
    Effect::new(move |_| crate::heads::install());
    crate::palette::install(ctx);
    // Back from the tray (or after the slideshow, the tray's Random, a
    // keybind...): what each monitor shows may have changed meanwhile.
    let _ = window_event_listener(leptos::ev::focus, move |_| {
        spawn_local(async move {
            ctx.refresh_state().await;
            ctx.refresh_catalog().await;
        });
    });
    let settings_open = RwSignal::new(false);

    spawn_local(async move {
        match api::list_monitors().await {
            Ok(monitors) => {
                let first = monitors
                    .iter()
                    .find(|m| m.focused)
                    .or_else(|| monitors.first())
                    .map(|m| m.name.clone());
                ctx.selection.set(first.into_iter().collect());
                ctx.monitors.set(monitors);
            }
            Err(e) => ctx.error_with("Could not list monitors (is Hyprland running?)", &e),
        }
        match api::get_settings().await {
            Ok(s) => ctx.settings.set(Some(s)),
            Err(e) => ctx.error_with("Could not load settings", &e),
        }
        ctx.refresh_state().await;
        ctx.refresh_catalog().await;
    });

    view! {
        <crate::palette::Backdrop />
        <Tabs default_value=TAB_LOCAL class="relative z-10 flex h-screen flex-col gap-0">
            // One bar: the tabs, then which monitors a click applies to.
            <header class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border/60 bg-background/55 px-5 py-2 backdrop-blur-xl">
                <span class="bg-gradient-to-r from-primary to-foreground/80 bg-clip-text text-xl font-black tracking-tight text-transparent">
                    "wallpicker"
                </span>
                <TabsList>
                    <TabsTrigger value=TAB_LOCAL>"Local"</TabsTrigger>
                    <TabsTrigger value=TAB_ONLINE>"Online"</TabsTrigger>
                    <TabsTrigger value=TAB_COLLECTIONS>"Collections"</TabsTrigger>
                </TabsList>
                <MonitorBar />
                <Tip text="Settings">
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Icon
                        attr:aria-label="Settings"
                        on_click=move |_| settings_open.set(true)
                    >
                        <CogIcon />
                    </Button>
                </Tip>
            </header>
            // Every panel stays mounted (inactive ones are only hidden), so
            // switching tabs keeps search results and scroll position.
            // Each tab is a header floating over its own scroll area
            // (heads.rs; `data-scroll-root`, `relative` so absolutely positioned content
            // like `sr-only` labels is clipped there instead of stretching
            // the page and scrolling the whole app away).
            <div class="relative flex min-h-0 flex-1 flex-col">
                <TabsContent value=TAB_LOCAL class="relative flex min-h-0 flex-col">
                    <LocalView />
                </TabsContent>
                <TabsContent value=TAB_ONLINE class="relative flex min-h-0 flex-col">
                    <OnlineView />
                </TabsContent>
                <TabsContent value=TAB_COLLECTIONS class="relative flex min-h-0 flex-col">
                    <CollectionsView />
                </TabsContent>
            </div>
            <SettingsDialog open=settings_open />
            <OpenPlaces settings_open=settings_open />
        </Tabs>
    }
}

/// Lucide's "settings" cog.
#[component]
fn CogIcon() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            class="size-4"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
        </svg>
    }
}

/// `wallpicker --open WHERE` (the Noctalia panel handing a job over): takes
/// the place at startup and whenever the backend says there's a new one,
/// then goes there. Duplicates and tags are Local's to open.
#[component]
fn OpenPlaces(settings_open: RwSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tabs = use_tabs();
    let take = move || {
        spawn_local(async move {
            if let Ok(Some(place)) = api::take_open_request().await {
                ctx.open_place.set(Some(place));
            }
        });
    };
    take();
    api::listen("open-request", take);
    Effect::new(move |_| {
        let Some(view) = ctx.open_place.with(|p| p.as_ref().map(|p| p.view.clone())) else { return };
        match view.as_str() {
            TAB_LOCAL | TAB_ONLINE | TAB_COLLECTIONS => {
                tabs.current_tab.set(view);
                ctx.open_place.set(None);
            }
            "settings" => {
                settings_open.set(true);
                ctx.open_place.set(None);
            }
            // Local opens these.
            _ => tabs.current_tab.set(TAB_LOCAL.to_string()),
        }
    });
}
