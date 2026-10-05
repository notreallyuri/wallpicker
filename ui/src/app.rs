//! The wallpicker UI, built on nocomponents.
//!
//! The styled components live in `src/components/nc/` (installed by
//! `cargo nocli`); the wallpaper tiles are plain markup styled with Tailwind
//! against the same theme tokens, so everything follows the light/dark mode.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::utils::theme::ThemeProvider;

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::monitor_bar::MonitorBar;
use crate::views::local::LocalView;
use crate::views::settings::SettingsDialog;
use crate::views::online::OnlineView;
use crate::views::online::collections::CollectionsView;

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
        <Tabs default_value=TAB_LOCAL class="flex h-screen flex-col gap-0">
            <header class="flex items-center gap-4 border-b px-5 py-3">
                <span class="text-base font-semibold tracking-tight">"wallpicker"</span>
                <TabsList>
                    <TabsTrigger value=TAB_LOCAL>"Local"</TabsTrigger>
                    <TabsTrigger value=TAB_ONLINE>"Online"</TabsTrigger>
                    <TabsTrigger value=TAB_COLLECTIONS>"Collections"</TabsTrigger>
                </TabsList>
                <Button
                    class="ml-auto"
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Icon
                    attr:aria-label="Settings"
                    attr:title="Settings"
                    on_click=move |_| settings_open.set(true)
                >
                    <CogIcon />
                </Button>
            </header>
            <MonitorBar />
            // Every panel stays mounted (inactive ones are only hidden), so
            // switching tabs keeps search results and scroll position.
            // `relative` makes this the containing block of absolutely
            // positioned content inside it (like `sr-only` labels), so it's
            // clipped here. Without it, such content at the bottom of a long
            // list stretches the page itself, and the whole app scrolls away.
            <div data-scroll-root class="relative min-h-0 flex-1 overflow-y-auto">
                <TabsContent value=TAB_LOCAL class="p-5">
                    <LocalView />
                </TabsContent>
                <TabsContent value=TAB_ONLINE class="p-5">
                    <OnlineView />
                </TabsContent>
                <TabsContent value=TAB_COLLECTIONS class="p-5">
                    <CollectionsView />
                </TabsContent>
            </div>
            <SettingsDialog open=settings_open />
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
