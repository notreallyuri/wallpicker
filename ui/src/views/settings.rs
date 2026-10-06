//! The Settings dialog, opened from the cog in the header.

use std::path::PathBuf;

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::utils::theme::{Theme, use_theme};
use nocomponents::primitives::sidebar::SidebarCollapsible;
use nocomponents::utils::types::Orientation;
use wallpicker_core::settings::{self, KonachanSite, Settings, WallpaperBackend};

use crate::api;
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;
use crate::views::Choice;
use crate::components::tip::Tip;


const SECTION_GENERAL: &str = "general";
const SECTION_APPEARANCE: &str = "appearance";
const SECTION_DOWNLOADS: &str = "downloads";
const SECTION_SLIDESHOW: &str = "slideshow";
const SECTION_BACKUP: &str = "backup";
const SECTION_WORKSPACES: &str = "workspaces";
const SECTION_PERFORMANCE: &str = "performance";
const SECTION_WALLHAVEN: &str = "wallhaven";
const SECTION_KONACHAN: &str = "konachan";
const SECTION_YANDERE: &str = "yandere";

struct Section {
    slug: &'static str,
    label: &'static str,
    group: &'static str,
    blurb: &'static str,
}

const GROUPS: &[&str] = &["App", "Providers"];

const SECTIONS: &[Section] = &[
    Section {
        slug: SECTION_GENERAL,
        label: "General",
        group: "App",
        blurb: "Where your wallpapers are, and how they're shown and applied.",
    },
    Section {
        slug: SECTION_APPEARANCE,
        label: "Appearance",
        group: "App",
        blurb: "How wallpicker looks.",
    },
    Section {
        slug: SECTION_DOWNLOADS,
        label: "Downloads",
        group: "App",
        blurb: "Where wallpapers from the online providers are saved.",
    },
    Section {
        slug: SECTION_SLIDESHOW,
        label: "Slideshow",
        group: "App",
        blurb: "A new wallpaper every few minutes, picked at random.",
    },
    Section {
        slug: SECTION_WORKSPACES,
        label: "Workspaces",
        group: "App",
        blurb: "A wallpaper for each Hyprland workspace, put on as you switch to it.",
    },
    Section {
        slug: SECTION_PERFORMANCE,
        label: "Performance",
        group: "App",
        blurb: "What wallpicker uses right now, and the effects that cost the most.",
    },
    Section {
        slug: SECTION_BACKUP,
        label: "Backup",
        group: "App",
        blurb: "Your favorites, tags and fits: copied every day on their own, and to a file when you want.",
    },
    Section {
        slug: SECTION_WALLHAVEN,
        label: "Wallhaven",
        group: "Providers",
        blurb: "wallhaven.cc",
    },
    Section {
        slug: SECTION_KONACHAN,
        label: "Konachan",
        group: "Providers",
        blurb: "Anime wallpapers, in a safe-for-work and a full version.",
    },
    Section {
        slug: SECTION_YANDERE,
        label: "yande.re",
        group: "Providers",
        blurb: "High-resolution anime scans.",
    },
];

const BACKENDS: &[(&str, &str)] = &settings::BACKENDS;

const KONACHAN_SITES: &[(&str, &str)] = &settings::KONACHAN_SITES;

const TRANSITIONS: &[(&str, &str)] = &settings::TRANSITIONS;

const THEMES: &[(&str, &str)] = &[("system", "Like the system"), ("light", "Off"), ("dark", "On")];

/// Color themes: `data-theme` name ("" is the base one), label, and the
/// background and accent of its light and dark halves, for the swatches.
/// The colors themselves are in globals.css.
const COLOR_THEMES: &[(&str, &str, [&str; 4])] = &[
    ("", "Blossom", ["oklch(0.975 0.012 75)", "oklch(0.62 0.16 350)", "oklch(0.2 0.022 305)", "oklch(0.8 0.11 345)"]),
    ("matcha", "Matcha", ["oklch(0.975 0.014 115)", "oklch(0.58 0.11 150)", "oklch(0.2 0.022 115)", "oklch(0.82 0.11 150)"]),
    ("peach", "Peach", ["oklch(0.975 0.014 55)", "oklch(0.64 0.15 38)", "oklch(0.2 0.022 55)", "oklch(0.82 0.11 38)"]),
    ("lagoon", "Lagoon", ["oklch(0.975 0.014 215)", "oklch(0.58 0.1 195)", "oklch(0.2 0.022 215)", "oklch(0.82 0.09 195)"]),
    ("lavender", "Lavender", ["oklch(0.975 0.014 295)", "oklch(0.58 0.15 290)", "oklch(0.2 0.022 295)", "oklch(0.8 0.11 290)"]),
    ("honey", "Honey", ["oklch(0.975 0.014 80)", "oklch(0.78 0.14 72)", "oklch(0.2 0.022 80)", "oklch(0.85 0.12 72)"]),
];

#[component]
pub(crate) fn SettingsDialog(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <Dialog open=open>
            // The form is built each time the dialog opens, from the saved
            // settings, so closing without saving drops the edits.
            // `p-0` because the panel goes to the edges; a fixed height because
            // the panel is `position: fixed`, pinned by this transformed box.
            <DialogPortal class="h-[34rem] max-h-[calc(100dvh-2rem)] gap-0 overflow-hidden p-0 sm:max-w-3xl">
                <SettingsForm on_saved=Callback::new(move |()| open.set(false)) />
            </DialogPortal>
        </Dialog>
    }
}

#[component]
fn SettingsForm(on_saved: Callback<()>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let theme = use_theme();

    let folders = RwSignal::new(String::new());
    let download_dir = RwSignal::new(String::new());
    let recursive = RwSignal::new(true);
    let close_to_tray = RwSignal::new(true);
    let theme_from_wallpaper = RwSignal::new(true);
    let full_previews = RwSignal::new(false);
    let ws_enabled = RwSignal::new(false);
    let ws_remember = RwSignal::new(true);
    // "" is the first monitor.
    let theme_monitor = RwSignal::new(String::new());
    let api_key = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let fit = RwSignal::new(String::new());
    // Monitor name -> its own fit ("cover", ...); absent: the general one.
    let monitor_fits = RwSignal::new(std::collections::BTreeMap::<String, String>::new());
    let konachan_site = RwSignal::new(KonachanSite::default().as_str().to_string());
    let backend = RwSignal::new(WallpaperBackend::default().as_str().to_string());
    // awww's transition; the numbers as typed.
    let transition = RwSignal::new(String::new());
    let transition_ms = RwSignal::new(String::new());
    let transition_fps = RwSignal::new(String::new());
    let thumb_width = RwSignal::new(String::new());
    let show = SlideshowForm::new();
    let saving = RwSignal::new(false);
    let current = RwSignal::new(SECTION_GENERAL);
    let showing = move || {
        let slug = current.get();
        SECTIONS.iter().find(|s| s.slug == slug).unwrap_or(&SECTIONS[0])
    };
    let sidebar_open = RwSignal::new(true);
    let base = StoredValue::new(None::<Settings>);

    // Theme is stored by nocomponents' ThemeProvider (localStorage), not in
    // wallpicker's config, so it applies immediately.
    let theme_choice = RwSignal::new(theme.current.get_untracked().as_str().to_string());
    Effect::new(move |_| {
        let next = match theme_choice.get().as_str() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        };
        theme.set_theme(next);
    });

    // Fill the form whenever settings are (re)loaded.
    Effect::new(move |_| {
        if let Some(s) = ctx.settings.get() {
            folders.set(
                s.folders
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            download_dir.set(s.download_dir.display().to_string());
            recursive.set(s.recursive);
            close_to_tray.set(s.close_to_tray);
            theme_from_wallpaper.set(s.theme_from_wallpaper);
            full_previews.set(s.full_previews);
            ws_enabled.set(s.workspaces.enabled);
            ws_remember.set(s.workspaces.remember);
            theme_monitor.set(s.theme_monitor.clone().unwrap_or_default());
            api_key.set(s.wallhaven_api_key.clone().unwrap_or_default());
            username.set(s.wallhaven_username.clone().unwrap_or_default());
            konachan_site.set(s.konachan_site.as_str().to_string());
            backend.set(s.wallpaper_backend.as_str().to_string());
            transition.set(s.transition.kind.clone());
            transition_ms.set(s.transition.duration_ms.to_string());
            transition_fps.set(s.transition.fps.to_string());
            fit.set(
                s.fit_mode
                    .map(|f| f.as_str().to_string())
                    .unwrap_or_default(),
            );
            monitor_fits.set(s.monitor_fits.iter().map(|(m, f)| (m.clone(), f.as_str().to_string())).collect());
            thumb_width.set(s.thumb_width.to_string());
            show.load(&s.slideshow);
            base.set_value(Some(s));
        }
    });

    let save = move || {
        let Some(mut s) = base.get_value() else {
            ctx.error("Settings have not loaded yet.");
            return;
        };

        let folder_list: Vec<PathBuf> = folders
            .get_untracked()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect();
        // `~` is expanded by the backend; anything else must be absolute.
        // `has_root`, not `is_absolute`: on wasm32-unknown-unknown
        // `is_absolute` is always false, so it would reject "/home/...".
        let relative: Vec<String> = folder_list
            .iter()
            .filter(|p| !p.has_root() && !p.starts_with("~"))
            .map(|p| p.display().to_string())
            .collect();
        if !relative.is_empty() {
            ctx.error_with(
                "Folders must be absolute paths (or start with ~)",
                &relative.join(", "),
            );
            return;
        }
        s.folders = folder_list;
        match show.read(s.slideshow.clone()) {
            Ok(sl) => s.slideshow = sl,
            Err(why) => {
                ctx.error(why);
                return;
            }
        }

        let dl = download_dir.get_untracked().trim().to_string();
        if !dl.is_empty() {
            let p = PathBuf::from(&dl);
            if !p.has_root() && !p.starts_with("~") {
                ctx.error("The download folder must be an absolute path (or start with ~).");
                return;
            }
            s.download_dir = p;
        }

        s.recursive = recursive.get_untracked();
        s.close_to_tray = close_to_tray.get_untracked();
        s.theme_from_wallpaper = theme_from_wallpaper.get_untracked();
        s.full_previews = full_previews.get_untracked();
        s.workspaces.enabled = ws_enabled.get_untracked();
        s.workspaces.remember = ws_remember.get_untracked();
        s.theme_monitor = Some(theme_monitor.get_untracked()).filter(|m| !m.is_empty());
        let key = api_key.get_untracked().trim().to_string();
        s.wallhaven_api_key = (!key.is_empty()).then_some(key);
        let name = username.get_untracked().trim().to_string();
        if !name.is_empty() && !wallpicker_core::wallhaven::is_valid_username(&name) {
            ctx.error("That isn't a Wallhaven username (letters, digits, _ - . only).");
            return;
        }
        s.wallhaven_username = (!name.is_empty()).then_some(name);
        s.wallpaper_backend = match backend.get_untracked().as_str() {
            "noctalia" => WallpaperBackend::Noctalia,
            "awww" => WallpaperBackend::Awww,
            _ => WallpaperBackend::Hyprpaper,
        };
        // Out-of-range numbers are brought into range when saved.
        s.transition.kind = transition.get_untracked();
        if let Ok(ms) = transition_ms.get_untracked().trim().parse() {
            s.transition.duration_ms = ms;
        }
        if let Ok(fps) = transition_fps.get_untracked().trim().parse() {
            s.transition.fps = fps;
        }
        s.konachan_site = match konachan_site.get_untracked().as_str() {
            "com" => KonachanSite::Com,
            _ => KonachanSite::Net,
        };
        s.fit_mode = crate::views::fit_from(&fit.get_untracked());
        s.monitor_fits = monitor_fits
            .get_untracked()
            .into_iter()
            .filter_map(|(m, f)| crate::views::fit_from(&f).map(|f| (m, f)))
            .collect();
        if let Ok(w) = thumb_width.get_untracked().trim().parse::<u32>() {
            s.thumb_width = w;
        }

        saving.set(true);
        spawn_local(async move {
            match api::save_settings(&s).await {
                Ok(saved) => {
                    ctx.settings.set(Some(saved));
                    ctx.library_rev.update(|r| *r += 1);
                    ctx.info("Settings saved");
                    on_saved.run(());
                }
                Err(e) => ctx.error_with("Could not save settings", &e),
            }
            // The form is gone if saving closed the dialog.
            saving.try_set(false);
        });
    };

    view! {
        // Laid out like nocomponents' "settings with the sidebar inside the
        // dialog" block: the panel picks the section, and the pane shows it.
        // The sidebar gets its own `open` signal; left uncontrolled it would
        // share the `ui-sidebar` storage key with any app-level sidebar.
        <SidebarProvider
            open=sidebar_open
            collapsible=SidebarCollapsible::None
            class="min-h-0"
            style="--sidebar-width: 12rem;"
        >
            <Sidebar>
                <SidebarHeader class="h-12 shrink-0 justify-center border-b">
                    <span class="truncate px-2 text-sm font-medium">"Settings"</span>
                </SidebarHeader>
                <SidebarContent>
                    {GROUPS
                        .iter()
                        .map(|group| {
                            view! {
                                <SidebarGroup>
                                    <SidebarGroupLabel>{*group}</SidebarGroupLabel>
                                    <SidebarGroupContent>
                                        <SidebarMenu>
                                            {SECTIONS
                                                .iter()
                                                .filter(|s| s.group == *group)
                                                .map(|s| {
                                                    let slug = s.slug;
                                                    view! {
                                                        <SidebarMenuItem>
                                                            <SidebarMenuButton
                                                                active=Signal::derive(move || current.get() == slug)
                                                                on:click=move |_| current.set(slug)
                                                            >
                                                                <span>{s.label}</span>
                                                            </SidebarMenuButton>
                                                        </SidebarMenuItem>
                                                    }
                                                })
                                                .collect_view()}
                                        </SidebarMenu>
                                    </SidebarGroupContent>
                                </SidebarGroup>
                            }
                        })
                        .collect_view()}
                </SidebarContent>
            </Sidebar>

            <SidebarInset>
                // The same height as the panel's header, so the two rules meet.
                // `pr-12` leaves the corner to the dialog's close button.
                <header class="flex h-12 shrink-0 items-center border-b px-4 pr-12">
                    // Mounted once and re-read, so `aria-labelledby` keeps
                    // pointing at it.
                    <DialogTitle class="text-sm">{move || showing().label}</DialogTitle>
                </header>
                <form
                    class="flex min-h-0 flex-1 flex-col"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        save();
                    }
                >
                    <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-4">
                        <DialogDescription>{move || showing().blurb}</DialogDescription>
                        // Only the picked section is built; its fields' signals
                        // live above, so switching keeps unsaved edits.
                        {move || match current.get() {
                            SECTION_GENERAL => view! {
                                <Field>
                                    <FieldLabel>"Wallpaper folders"</FieldLabel>
                                    <Textarea model=folders class="font-mono text-xs" attr:rows="4" attr:spellcheck="false" />
                                    <FieldDescription>"One folder per line. ~ is allowed."</FieldDescription>
                                </Field>
                                <Field orientation=Orientation::Horizontal>
                                    <Switch checked=recursive />
                                    <FieldLabel>"Include subfolders"</FieldLabel>
                                </Field>
                                <Field orientation=Orientation::Horizontal>
                                    <Switch checked=close_to_tray />
                                    <FieldLabel>"Keep running in the tray when the window is closed"</FieldLabel>
                                </Field>
                                <Field>
                                    <FieldLabel>"Wallpaper backend"</FieldLabel>
                                    <Choice items=BACKENDS.to_vec() model=backend />
                                    <FieldDescription>"What draws the wallpaper; wallpicker applies through it."</FieldDescription>
                                </Field>
                                <Conflicts backend=backend />
                                <Show when=move || backend.get() == WallpaperBackend::Awww.as_str()>
                                    <p class="text-sm text-muted-foreground">
                                        "awww (called swww before) draws the wallpaper with a transition and keeps it "
                                        "after a restart. wallpicker starts its daemon when it isn't running; to have "
                                        "it from login, add awww-daemon to Hyprland's autostart."
                                    </p>
                                    <div class="grid gap-6 sm:grid-cols-3">
                                        <Field>
                                            <FieldLabel>"Transition"</FieldLabel>
                                            <Choice items=TRANSITIONS.to_vec() model=transition />
                                        </Field>
                                        <Field>
                                            <FieldLabel>"Duration (ms)"</FieldLabel>
                                            <Input
                                                model=transition_ms
                                                input_type=InputType::Number
                                                attr:min="0"
                                                attr:max="10000"
                                                attr:step="100"
                                            />
                                        </Field>
                                        <Field>
                                            <FieldLabel>"Frames per second"</FieldLabel>
                                            <Input
                                                model=transition_fps
                                                input_type=InputType::Number
                                                attr:min="1"
                                                attr:max="255"
                                            />
                                            <FieldDescription>"Your monitors' refresh rate is smoothest."</FieldDescription>
                                        </Field>
                                    </div>
                                </Show>
                                <Show when=move || backend.get() == WallpaperBackend::Noctalia.as_str()>
                                    <p class="text-sm text-muted-foreground">
                                        "Noctalia keeps the wallpapers itself, so they come back after a restart. "
                                        "Its fill mode and transitions are set in Noctalia's settings (Wallpaper). "
                                        "Turn Noctalia's wallpaper on there, and stop hyprpaper (take it out of "
                                        "your Hyprland autostart): both draw the background."
                                    </p>
                                </Show>
                                <div class="grid gap-6 sm:grid-cols-2">
                                    // Noctalia keeps its own fill mode.
                                    <Show when=move || backend.get() != WallpaperBackend::Noctalia.as_str()>
                                        <Field>
                                            <FieldLabel>"Fit mode"</FieldLabel>
                                            // Labels as the picked backend does them.
                                            {move || {
                                                let b = match backend.get().as_str() {
                                                    "awww" => WallpaperBackend::Awww,
                                                    _ => WallpaperBackend::Hyprpaper,
                                                };
                                                view! { <Choice items=settings::fit_modes(b).to_vec() model=fit /> }
                                            }}
                                            <FieldDescription>
                                                {move || {
                                                    if backend.get() == WallpaperBackend::Awww.as_str() {
                                                        "For every monitor, unless it has its own below; a monitor's Adjust (the drawer) wins over both, until its wallpaper changes."
                                                    } else {
                                                        "For every monitor, unless it has its own below; a monitor's Adjust (the drawer) wins over both, until its wallpaper changes. Needs a newer hyprpaper: leave it on default if applying fails."
                                                    }
                                                }}
                                            </FieldDescription>
                                        </Field>
                                        <Show when=move || ctx.monitors.with(|m| !m.is_empty())>
                                            <Field>
                                                <FieldLabel>"Fit per monitor"</FieldLabel>
                                                <div class="flex flex-col gap-2">
                                                    <For
                                                        each=move || ctx.monitors.get()
                                                        key=|m| m.name.clone()
                                                        children=move |m: wallpicker_core::monitors::Monitor| {
                                                            view! { <MonitorFit monitor=m fits=monitor_fits /> }
                                                        }
                                                    />
                                                </div>
                                            </Field>
                                        </Show>
                                    </Show>
                                    <Field>
                                        <FieldLabel>"Thumbnail width (px)"</FieldLabel>
                                        <Input
                                            model=thumb_width
                                            input_type=InputType::Number
                                            attr:min="120"
                                            attr:max="1024"
                                            attr:step="20"
                                        />
                                    </Field>
                                </div>
                            }
                            .into_any(),
                            SECTION_APPEARANCE => view! {
                                <Field>
                                    <FieldLabel>"Dark mode"</FieldLabel>
                                    <Choice items=THEMES.to_vec() model=theme_choice />
                                    <FieldDescription>"Applies right away."</FieldDescription>
                                </Field>
                                <ColorThemes following=theme_from_wallpaper />
                                <Field orientation=Orientation::Horizontal>
                                    <Switch checked=theme_from_wallpaper />
                                    <FieldLabel>"Colors from the wallpaper"</FieldLabel>
                                </Field>
                                // Plain text: a FieldDescription needs to sit inside a Field.
                                <p class="-mt-2 text-sm text-muted-foreground">
                                    "The app takes its colors from a monitor's wallpaper, the way Noctalia "
                                    "makes them (needs Noctalia installed). Save to apply."
                                </p>
                                <Show when=move || theme_from_wallpaper.get() && ctx.monitors.with(|m| m.len() > 1)>
                                    <ColorsMonitor model=theme_monitor />
                                </Show>
                                <NoctaliaColorsField />
                                <Field orientation=Orientation::Horizontal>
                                    <Switch checked=ctx.backdrop />
                                    <FieldLabel>"Wallpaper behind the app"</FieldLabel>
                                </Field>
                                <p class="-mt-2 text-sm text-muted-foreground">
                                    "Your current wallpaper, blurred, as the app's background. Applies right away."
                                </p>
                            }
                            .into_any(),
                            SECTION_DOWNLOADS => view! {
                                <Field>
                                    <FieldLabel>"Download folder"</FieldLabel>
                                    <Input model=download_dir class="font-mono text-xs" attr:spellcheck="false" />
                                    <FieldDescription>
                                        "Where wallpapers from every online provider are saved, as "
                                        "wallhaven-<id> and konachan-<id> files. Put it inside one of your "
                                        "wallpaper folders to see downloads in the Local tab."
                                    </FieldDescription>
                                </Field>
                            }
                            .into_any(),
                            SECTION_SLIDESHOW => view! { <SlideshowPane form=show /> }.into_any(),
                            SECTION_BACKUP => view! { <BackupPane /> }.into_any(),
                            SECTION_WORKSPACES => {
                                view! { <super::workspaces::WorkspacesPane enabled=ws_enabled remember=ws_remember /> }.into_any()
                            }
                            SECTION_PERFORMANCE => view! { <PerformancePane full_previews=full_previews /> }.into_any(),
                            SECTION_WALLHAVEN => view! {
                                <Field>
                                    <FieldLabel>"API key"</FieldLabel>
                                    <Input
                                        model=api_key
                                        input_type=InputType::Password
                                        attr:placeholder="Optional"
                                        attr:autocomplete="off"
                                    />
                                    <FieldDescription>
                                        "Needed for NSFW results and your collections. Find it at wallhaven.cc → "
                                        "Settings → Account. Stored in your config file, readable only by you."
                                    </FieldDescription>
                                </Field>
                                <Field>
                                    <FieldLabel>"Username"</FieldLabel>
                                    <Input model=username attr:placeholder="Optional" attr:autocomplete="off" attr:spellcheck="false" />
                                    <FieldDescription>
                                        "Your Wallhaven username, for the Collections tab: opening a collection "
                                        "needs it (the API doesn't say whose key it is)."
                                    </FieldDescription>
                                </Field>
                            }
                            .into_any(),
                            SECTION_KONACHAN => view! {
                                <Field>
                                    <FieldLabel>"Site"</FieldLabel>
                                    <Choice items=KONACHAN_SITES.to_vec() model=konachan_site />
                                    <FieldDescription>
                                        "Same posts on both. konachan.net only has safe-for-work ones; "
                                        "konachan.com adds the Questionable and Explicit ratings, which the "
                                        "Filters dialog then offers. No account or key is needed."
                                    </FieldDescription>
                                </Field>
                                <p class="text-sm text-muted-foreground">
                                    "Konachan doesn't mark artist tags in its API, so downloads from it "
                                    "aren't tagged with their artist."
                                </p>
                            }
                            .into_any(),
                            _ => view! {
                                <p class="text-sm">"Nothing to set up: no account or key is needed."</p>
                                <p class="text-sm text-muted-foreground">
                                    "yande.re has all ratings (Safe only until you tick others in the "
                                    "Filters dialog) and many very large scans. Originals over 64 MiB are "
                                    "downloaded as yande.re's full-resolution JPEG instead. Like Konachan, "
                                    "its API doesn't mark artist tags."
                                </p>
                            }
                            .into_any(),
                        }}
                    </div>
                    <footer class="flex shrink-0 items-center justify-end gap-2 border-t px-4 py-3">
                        <span class="mr-auto text-xs text-muted-foreground">
                            "Saved to ~/.config/wallpicker/config.json"
                        </span>
                        <Button r#type=ButtonType::Submit disabled=saving>
                            {move || if saving.get() { "Saving…" } else { "Save" }}
                        </Button>
                    </footer>
                </form>
            </SidebarInset>
        </SidebarProvider>
    }
}

/// One monitor's own fit: "Same as above" (the general fit) or a fit.
#[component]
fn MonitorFit(
    monitor: wallpicker_core::monitors::Monitor,
    fits: RwSignal<std::collections::BTreeMap<String, String>>,
) -> impl IntoView {
    let name = monitor.name.clone();
    let model = RwSignal::new(fits.with_untracked(|f| f.get(&name).cloned().unwrap_or_default()));
    // Follow the loaded settings, and write choices back.
    let read = name.clone();
    Effect::new(move |_| {
        let v = fits.with(|f| f.get(&read).cloned().unwrap_or_default());
        if model.with_untracked(|m| *m != v) {
            model.set(v);
        }
    });
    let write = name.clone();
    Effect::new(move |_| {
        let v = model.get();
        fits.update(|f| {
            if v.is_empty() {
                f.remove(&write);
            } else {
                f.insert(write.clone(), v);
            }
        });
    });
    view! {
        <div class="flex items-center gap-3">
            <span class="w-40 text-sm">
                {monitor.name.clone()}
                <span class="ml-2 text-xs text-muted-foreground">{format!("{}×{}", monitor.width, monitor.height)}</span>
            </span>
            <Choice items=MONITOR_FITS.to_vec() model=model class="w-48" />
        </div>
    }
}

/// A monitor's own fit: "" follows the general one.
const MONITOR_FITS: &[(&str, &str)] = &[
    ("", "Same as above"),
    ("cover", "Cover"),
    ("contain", "Contain"),
    ("tile", "Tile"),
    ("fill", "Fill"),
];

/// The Slideshow section's fields, read into `Settings.slideshow` on Save.
#[derive(Clone, Copy)]
struct SlideshowForm {
    enabled: RwSignal<bool>,
    minutes: RwSignal<String>,
    day: PickForm,
    monitors: RwSignal<Vec<String>>,
    each: RwSignal<bool>,
    night_enabled: RwSignal<bool>,
    night_from: RwSignal<String>,
    night_to: RwSignal<String>,
    night: PickForm,
}

/// What the slideshow picks from: the day's set, or the night's.
#[derive(Clone, Copy)]
struct PickForm {
    favorites: RwSignal<bool>,
    tags: RwSignal<Vec<String>>,
    any: RwSignal<bool>,
    folder: RwSignal<String>,
    /// "" is any color.
    color: RwSignal<String>,
}

/// A set's picks as read from the form: favorites, tags, any, folder, color.
type Picks = (bool, Vec<String>, bool, Option<PathBuf>, Option<String>);

impl PickForm {
    fn new() -> PickForm {
        PickForm {
            favorites: RwSignal::new(false),
            tags: RwSignal::new(Vec::new()),
            any: RwSignal::new(false),
            folder: RwSignal::new(String::new()),
            color: RwSignal::new(String::new()),
        }
    }

    fn load(self, favorites: bool, tags: &[String], any: bool, folder: Option<&PathBuf>, color: Option<&String>) {
        self.favorites.set(favorites);
        self.tags.set(tags.to_vec());
        self.any.set(any);
        self.folder.set(folder.map(|p| p.display().to_string()).unwrap_or_default());
        self.color.set(color.cloned().unwrap_or_default());
    }

    fn read(self) -> Result<Picks, &'static str> {
        let folder = self.folder.get_untracked().trim().to_string();
        let folder = if folder.is_empty() {
            None
        } else {
            let p = PathBuf::from(&folder);
            // `has_root`: `is_absolute` is always false on wasm.
            if !p.has_root() && !p.starts_with("~") {
                return Err("A slideshow folder must be an absolute path (or start with ~).");
            }
            Some(p)
        };
        let color = Some(self.color.get_untracked()).filter(|c| !c.is_empty());
        Ok((self.favorites.get_untracked(), self.tags.get_untracked(), self.any.get_untracked(), folder, color))
    }
}

/// The color choices for a set: any, the hues, then the tones.
fn color_choices() -> Vec<(&'static str, &'static str)> {
    use wallpicker_core::colors::{HUES, TONES};
    std::iter::once(("", "Any color")).chain(HUES).chain(TONES).collect()
}

const SLIDESHOW_MINUTES: &[(&str, &str)] = &[
    ("5", "5 minutes"),
    ("10", "10 minutes"),
    ("15", "15 minutes"),
    ("30", "30 minutes"),
    ("60", "1 hour"),
    ("120", "2 hours"),
    ("240", "4 hours"),
    ("480", "8 hours"),
    ("1440", "24 hours"),
];

impl SlideshowForm {
    fn new() -> SlideshowForm {
        SlideshowForm {
            enabled: RwSignal::new(false),
            minutes: RwSignal::new("30".into()),
            day: PickForm::new(),
            monitors: RwSignal::new(Vec::new()),
            each: RwSignal::new(true),
            night_enabled: RwSignal::new(false),
            night_from: RwSignal::new("20:00".into()),
            night_to: RwSignal::new("07:00".into()),
            night: PickForm::new(),
        }
    }

    fn load(self, s: &settings::Slideshow) {
        self.enabled.set(s.enabled);
        self.minutes.set(s.minutes.to_string());
        self.day.load(s.favorites, &s.tags, s.any, s.folder.as_ref(), s.color.as_ref());
        self.monitors.set(s.monitors.clone());
        self.each.set(s.each);
        let n = &s.night;
        self.night_enabled.set(n.enabled);
        self.night_from.set(n.from.clone());
        self.night_to.set(n.to.clone());
        self.night.load(n.favorites, &n.tags, n.any, n.folder.as_ref(), n.color.as_ref());
    }

    fn read(self, mut s: settings::Slideshow) -> Result<settings::Slideshow, &'static str> {
        s.enabled = self.enabled.get_untracked();
        s.minutes = self.minutes.get_untracked().parse().unwrap_or(30).clamp(1, 24 * 60);
        (s.favorites, s.tags, s.any, s.folder, s.color) = self.day.read()?;
        s.monitors = self.monitors.get_untracked();
        s.each = self.each.get_untracked();
        let n = &mut s.night;
        n.enabled = self.night_enabled.get_untracked();
        for (signal, field) in [(self.night_from, &mut n.from), (self.night_to, &mut n.to)] {
            let t = signal.get_untracked();
            if settings::parse_hhmm(&t).is_none() {
                return Err("The night's times must look like 20:00.");
            }
            *field = t.trim().to_string();
        }
        (n.favorites, n.tags, n.any, n.folder, n.color) = self.night.read()?;
        Ok(s)
    }
}

#[component]
fn SlideshowPane(form: SlideshowForm) -> impl IntoView {
    use nocomponents::primitives::toggle_group::ToggleGroupType;
    let ctx = expect_context::<Ctx>();
    // Tags in the library, for the chips.
    let tags = Memo::new(move |_| {
        ctx.catalog.with(|c| {
            c.tag_counts(c.items.keys().map(|p| p.as_path()))
                .into_iter()
                .map(|(t, _)| t)
                .collect::<Vec<_>>()
        })
    });
    let status = RwSignal::new(None::<String>);
    let refresh = move || {
        spawn_local(async move {
            let text = match api::slideshow_status().await {
                Ok(s) if s.enabled => s.next_in.map(|n| {
                    let when = match s.period.as_deref() {
                        Some("night") => "Night now. ",
                        Some(_) => "Day now. ",
                        None => "",
                    };
                    format!("{when}Next change in about {} min.", n.div_ceil(60).max(1))
                }),
                Ok(_) => Some("Off.".to_string()),
                Err(_) => None,
            };
            status.try_set(text);
        });
    };
    refresh();
    let busy = RwSignal::new(false);
    let change_now = move |_| {
        busy.set(true);
        spawn_local(async move {
            match api::slideshow_now().await {
                Ok(()) => ctx.info("New wallpaper"),
                Err(e) => ctx.error_with("The slideshow could not change it", &e),
            }
            busy.try_set(false);
            ctx.refresh_state().await;
            refresh();
        });
    };
    view! {
        <Field orientation=Orientation::Horizontal>
            <Switch checked=form.enabled />
            <FieldLabel>"Change the wallpaper by itself"</FieldLabel>
        </Field>
        <Field>
            <FieldLabel>"Every"</FieldLabel>
            <Choice items=SLIDESHOW_MINUTES.to_vec() model=form.minutes class="w-48" />
        </Field>
        <PickFields set=form.day tags=tags />
        <Show when=move || ctx.monitors.with(|m| m.len() > 1)>
            <Field>
                <FieldLabel>"On these monitors"</FieldLabel>
                <ToggleGroup
                    kind=ToggleGroupType::Multiple
                    variant=ToggleVariant::Outline
                    size=ToggleSize::Sm
                    value=form.monitors
                    class="flex-wrap gap-1.5"
                >
                    <For
                        each=move || ctx.monitors.get()
                        key=|m| m.name.clone()
                        children=|m: wallpicker_core::monitors::Monitor| {
                            view! { <ToggleGroupItem value=m.name.clone() class="px-3">{m.name.clone()}</ToggleGroupItem> }
                        }
                    />
                </ToggleGroup>
                <FieldDescription>"None picked: all of them."</FieldDescription>
            </Field>
            <Field orientation=Orientation::Horizontal>
                <Switch checked=form.each />
                <FieldLabel>"A different wallpaper on each monitor"</FieldLabel>
            </Field>
        </Show>
        <div class="flex flex-col gap-4 rounded-2xl border-2 border-border/60 p-4">
            <Field orientation=Orientation::Horizontal>
                <Switch checked=form.night_enabled />
                <FieldLabel>"Something else at night"</FieldLabel>
            </Field>
            <Show when=move || form.night_enabled.get()>
                <div class="flex items-center gap-2 text-sm">
                    <span>"From"</span>
                    <Input model=form.night_from input_type=InputType::Time class="w-32" />
                    <span>"to"</span>
                    <Input model=form.night_to input_type=InputType::Time class="w-32" />
                </div>
                <PickFields set=form.night tags=tags />
                <p class="text-xs text-muted-foreground">
                    "It changes right away when night starts and ends, then every interval as usual."
                </p>
            </Show>
        </div>
        <div class="flex items-center gap-3">
            <Button variant=ButtonVariant::Outline disabled=busy on_click=change_now>
                "Change now"
            </Button>
            <span class="text-sm text-muted-foreground">{move || status.get()}</span>
        </div>
        <p class="text-xs text-muted-foreground">
            "Kept on time by the Noctalia plugin while Noctalia runs, or by "
            <code>"wallpicker slideshow run"</code>
            " (for a Hyprland autostart). Save to apply changes here."
        </p>
    }
}

/// Noctalia's colors follow the wallpaper, or its own scheme. Applies at
/// once (it's Noctalia's setting, not wallpicker's).
#[component]
fn NoctaliaColorsField() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let status = RwSignal::new(None::<api::NoctaliaColors>);
    spawn_local(async move {
        status.try_set(api::noctalia_colors().await.ok());
    });
    let following = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(s) = status.get() {
            following.set(s.following);
        }
    });
    // A flip of the switch: tell Noctalia.
    Effect::new(move |prev: Option<bool>| {
        let want = following.get();
        let known = status.with_untracked(|s| s.as_ref().map(|s| s.following));
        if prev.is_some() && known.is_some() && known != Some(want) {
            spawn_local(async move {
                match api::set_noctalia_colors(want).await {
                    Ok(s) => {
                        status.set(Some(s));
                    }
                    Err(e) => {
                        ctx.error_with("Could not change Noctalia's colors", &e);
                        if let Ok(s) = api::noctalia_colors().await {
                            status.set(Some(s));
                        }
                    }
                }
            });
        }
        want
    });
    view! {
        <Show when=move || status.with(|s| s.as_ref().is_some_and(|s| s.available))>
            <Field orientation=Orientation::Horizontal>
                <Switch checked=following />
                <FieldLabel>"Noctalia's colors follow the wallpaper"</FieldLabel>
            </Field>
            <p class="-mt-2 text-sm text-muted-foreground">
                {move || {
                    let now = status.with(|s| s.as_ref().and_then(|s| s.scheme.clone()).unwrap_or_default());
                    format!(
                        "Applies at once. Now: {now}. Turning it off goes back to the scheme Noctalia had before."
                    )
                }}
            </p>
        </Show>
    }
}

/// The color themes, as swatches; applies at once (stored like dark mode).
/// Wallpaper colors win while they're on.
#[component]
fn ColorThemes(following: RwSignal<bool>) -> impl IntoView {
    let theme = use_theme();
    view! {
        <div class="flex flex-col gap-2">
            <span class="text-sm font-bold">"Color theme"</span>
            <div class=move || {
                format!(
                    "grid grid-cols-3 gap-2 transition-opacity {}",
                    if following.get() { "opacity-50" } else { "" },
                )
            }>
                {COLOR_THEMES
                    .iter()
                    .map(|&(name, label, colors)| {
                        let picked = move || theme.color.with(|c| c == name);
                        let swatch = move || {
                            let (bg, accent) = if theme.is_dark() { (colors[2], colors[3]) } else { (colors[0], colors[1]) };
                            format!("background: {bg}; --swatch: {accent};")
                        };
                        view! {
                            <button
                                type="button"
                                on:click=move |_| theme.set_color(name)
                                class=move || {
                                    format!(
                                        "flex items-center gap-2.5 rounded-2xl border-2 p-2 text-left text-sm font-bold transition duration-200 ease-bounce hover:-translate-y-0.5 {}",
                                        if picked() { "border-primary bg-primary/10" } else { "border-border hover:border-primary/40" },
                                    )
                                }
                            >
                                <span
                                    class="flex size-8 shrink-0 items-center justify-center rounded-xl shadow-inner ring-1 ring-foreground/10"
                                    style=swatch
                                >
                                    <span class="size-4 rounded-full bg-(--swatch) shadow-[0_2px_0_0_rgb(0_0_0/0.15)]" />
                                </span>
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <p class="text-sm text-muted-foreground">
                {move || {
                    if following.get() {
                        "Colors from the wallpaper are on, so this one shows when they're off."
                    } else {
                        "Applies right away, light and dark."
                    }
                }}
            </p>
        </div>
    }
}

/// Which monitor's wallpaper the colors come from ("" is the first one).
#[component]
fn ColorsMonitor(model: RwSignal<String>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    // A Button's variant is fixed, so each pill is rebuilt when the choice changes.
    let pill = move |value: String, label: String| {
        move || {
            let variant = if model.with(|m| *m == value) { ButtonVariant::Default } else { ButtonVariant::Outline };
            let value = value.clone();
            let label = label.clone();
            view! {
                <Button size=ButtonSize::Sm variant=variant on_click=move |_| model.set(value.clone())>
                    {label.clone()}
                </Button>
            }
        }
    };
    view! {
        <div class="flex flex-wrap items-center gap-2">
            <span class="text-sm font-bold">"From"</span>
            {pill(String::new(), "First monitor".into())}
            {move || {
                ctx.monitors
                    .get()
                    .into_iter()
                    .map(|m| pill(m.name.clone(), m.name).into_any())
                    .collect_view()
            }}
        </div>
    }
}

/// A warning when something besides the picked backend draws wallpapers
/// too (hyprpaper still running, Noctalia's own wallpaper on...): they fight
/// over the screen. Checked again whenever the backend is changed; each
/// program can be stopped from here (Noctalia's wallpaper is switched off in
/// Noctalia's settings, so that one only says how).
#[component]
fn Conflicts(backend: RwSignal<String>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let found = RwSignal::new(Vec::<String>::new());
    let check = RwSignal::new(0u32);
    Effect::new(move |_| {
        check.track();
        let b = backend.get();
        if b.is_empty() {
            return;
        }
        spawn_local(async move {
            let list = api::wallpaper_conflicts(&b).await.unwrap_or_default();
            if backend.get_untracked() == b {
                found.try_set(list);
            }
        });
    });
    let stopping = RwSignal::new(None::<String>);
    let stop = move |name: String| {
        stopping.set(Some(name.clone()));
        spawn_local(async move {
            match api::stop_wallpaper_program(&name).await {
                Ok(()) => ctx.info(format!(
                    "Stopped {name}. If it's in Hyprland's autostart, take it out there or it starts again at login."
                )),
                Err(e) => ctx.error_with(&format!("Could not stop {name}"), &e),
            }
            stopping.try_set(None);
            // What's still running now.
            check.update(|n| *n += 1);
        });
    };
    view! {
        <Show when=move || found.with(|f| !f.is_empty())>
            <div class="flex flex-col gap-2 rounded-2xl border-2 border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm">
                <p class="font-bold text-amber-600 dark:text-amber-400">"Also drawing wallpapers"</p>
                <p class="text-muted-foreground">"Only one program should: they cover each other."</p>
                <For
                    each=move || found.get()
                    key=|name| name.clone()
                    children=move |name: String| {
                        let noctalia = name.starts_with("Noctalia");
                        let busy_name = name.clone();
                        let stop_name = name.clone();
                        view! {
                            <div class="flex items-center gap-3">
                                <span class="font-bold">{name.clone()}</span>
                                {if noctalia {
                                    view! {
                                        <span class="text-muted-foreground">
                                            "Turn it off in Noctalia's settings (Wallpaper)."
                                        </span>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <Button
                                            class="ml-auto"
                                            size=ButtonSize::Sm
                                            variant=ButtonVariant::Outline
                                            disabled=Signal::derive(move || stopping.with(|s| s.as_deref() == Some(busy_name.as_str())))
                                            on_click=move |_| stop(stop_name.clone())
                                        >
                                            "Stop it"
                                        </Button>
                                    }
                                        .into_any()
                                }}
                            </div>
                        }
                    }
                />
            </div>
        </Show>
    }
}

/// Settings → Backup: export, import, and the daily snapshots. These act
/// at once (not with Save): they're about the library, not the settings.
/// Export and import go through the system's file dialogs.
#[component]
fn BackupPane() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let replace = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let report = RwSignal::new(None::<String>);
    let snapshots = RwSignal::new(None::<Vec<api::Snapshot>>);
    let folder = RwSignal::new(String::new());
    // The snapshot whose Restore was pressed once (asks before replacing).
    let confirming = RwSignal::new(None::<String>);
    let reload = move || {
        spawn_local(async move {
            snapshots.try_set(Some(api::backup_snapshots().await.unwrap_or_default()));
            if let Ok(f) = api::snapshot_dir().await {
                folder.try_set(f);
            }
        });
    };
    reload();

    let export = move |_| {
        busy.set(true);
        spawn_local(async move {
            match api::backup_export_pick().await {
                Ok(Some(e)) => ctx.info(format!("Exported {} wallpapers to {}", e.items, e.path)),
                Ok(None) => {}
                Err(e) => ctx.error_with("Could not export", &e),
            }
            busy.try_set(false);
        });
    };
    let import = move |_| {
        busy.set(true);
        let replace = replace.get_untracked();
        spawn_local(async move {
            match api::backup_import_pick(replace).await {
                Ok(Some(r)) => {
                    let mut text = format!("Found {} where they were, {} moved.", r.same_place, r.moved);
                    if !r.missing.is_empty() {
                        text.push_str(&format!(" Not in your library now ({}): {}.", r.missing.len(), r.missing.join(", ")));
                    }
                    report.try_set(Some(text));
                    ctx.refresh_catalog().await;
                    reload();
                }
                Ok(None) => {}
                Err(e) => ctx.error_with("Could not import", &e),
            }
            busy.try_set(false);
        });
    };
    let move_folder = move |reset: bool| {
        busy.set(true);
        spawn_local(async move {
            match api::snapshot_dir_pick(reset).await {
                Ok(Some(dir)) => {
                    ctx.info(format!("Snapshots now go to {dir}"));
                    reload();
                }
                Ok(None) => {}
                Err(e) => ctx.error_with("Could not use that folder", &e),
            }
            busy.try_set(false);
        });
    };
    let restore = move |id: String| {
        busy.set(true);
        spawn_local(async move {
            match api::backup_restore(&id).await {
                Ok(c) => {
                    ctx.info(format!("Library restored ({} wallpapers)", c.items.len()));
                    ctx.refresh_catalog().await;
                }
                Err(e) => ctx.error_with("Could not restore", &e),
            }
            confirming.try_set(None);
            busy.try_set(false);
            reload();
        });
    };

    view! {
        <div class="flex flex-col gap-6">
            <div class="flex flex-col gap-2">
                <span class="text-sm font-bold">"Export and import"</span>
                <div class="flex flex-wrap items-center gap-2">
                    <Button disabled=busy on_click=export>"Export…"</Button>
                    <Button variant=ButtonVariant::Outline disabled=busy on_click=import>"Import…"</Button>
                </div>
                <p class="text-sm text-muted-foreground">
                    "Favorites, tags and fits, in one file. An import is merged into your library: a "
                    "wallpaper that moved (another folder, another home) is found by its place in your "
                    "wallpaper folders, or by name and size."
                </p>
                <Field orientation=Orientation::Horizontal>
                    <Switch checked=replace />
                    <FieldLabel>"Replace the library instead of merging"</FieldLabel>
                </Field>
                {move || report.get().map(|r| view! { <p class="rounded-xl bg-primary/10 px-3 py-2 text-sm">{r}</p> })}
            </div>
            <div class="flex flex-col gap-2">
                <span class="text-sm font-bold">"Daily snapshots"</span>
                <p class="text-sm text-muted-foreground">
                    "A copy of the library before the first change of each day; the last 14 are kept. "
                    "Restoring keeps what's there now as a snapshot too. A synced folder (Syncthing, "
                    "Nextcloud…) makes them backups off this machine."
                </p>
                <div class="flex flex-wrap items-center gap-2 text-sm">
                    <span class="text-muted-foreground">"Saved in"</span>
                    <Tip text=folder class="min-w-0 max-w-full">
                        <code class="block truncate rounded-lg bg-muted px-2 py-1 text-xs">{move || folder.get()}</code>
                    </Tip>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline disabled=busy on_click=move |_| move_folder(false)>
                        "Change…"
                    </Button>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost disabled=busy on_click=move |_| move_folder(true)>
                        "Default"
                    </Button>
                </div>
                {move || match snapshots.get() {
                    None => view! { <Spinner /> }.into_any(),
                    Some(list) if list.is_empty() => {
                        view! { <p class="text-sm text-muted-foreground">"None yet: one is made before your next change."</p> }
                            .into_any()
                    }
                    Some(list) => list
                        .into_iter()
                        .map(|s| {
                            let id = s.id.clone();
                            let id_ask = s.id.clone();
                            let id_cancel = s.id.clone();
                            let asking = Memo::new(move |_| confirming.get().as_deref() == Some(id_cancel.as_str()));
                            view! {
                                <div class="flex items-center gap-3 rounded-xl border-2 px-3 py-1.5 text-sm">
                                    <span class="font-bold tabular-nums">{s.label.clone()}</span>
                                    <span class="text-muted-foreground">
                                        {format!("{} wallpaper{}", s.items, if s.items == 1 { "" } else { "s" })}
                                    </span>
                                    <span class="ml-auto flex items-center gap-2">
                                        <Show
                                            when=move || asking.get()
                                            fallback=move || {
                                                let id_ask = id_ask.clone();
                                                view! {
                                                    <Button
                                                        size=ButtonSize::Sm
                                                        variant=ButtonVariant::Ghost
                                                        disabled=busy
                                                        on_click=move |_| confirming.set(Some(id_ask.clone()))
                                                    >
                                                        "Restore"
                                                    </Button>
                                                }
                                            }
                                        >
                                            <span class="text-muted-foreground">"Replace the library with this?"</span>
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| confirming.set(None)>
                                                "Cancel"
                                            </Button>
                                            {
                                                let id = id.clone();
                                                view! {
                                                    <Button
                                                        size=ButtonSize::Sm
                                                        variant=ButtonVariant::Destructive
                                                        disabled=busy
                                                        on_click=move |_| restore(id.clone())
                                                    >
                                                        "Restore"
                                                    </Button>
                                                }
                                            }
                                        </Show>
                                    </span>
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any(),
                }}
            </div>
        </div>
    }
}

/// One set's picks: favorites, color, tags (with all/any) and folder.
#[component]
fn PickFields(set: PickForm, tags: Memo<Vec<String>>) -> impl IntoView {
    use nocomponents::primitives::toggle_group::ToggleGroupType;
    view! {
        <Field orientation=Orientation::Horizontal>
            <Switch checked=set.favorites />
            <FieldLabel>"Only favorites"</FieldLabel>
        </Field>
        <Field>
            <FieldLabel>"Only with the color"</FieldLabel>
            <Choice items=color_choices() model=set.color class="w-48" />
        </Field>
        <Show when=move || tags.with(|t| !t.is_empty())>
            <Field>
                <FieldLabel>"Only with these tags"</FieldLabel>
                <ToggleGroup
                    kind=ToggleGroupType::Multiple
                    variant=ToggleVariant::Outline
                    size=ToggleSize::Sm
                    value=set.tags
                    class="flex-wrap gap-1.5"
                >
                    <For
                        each=move || tags.get()
                        key=|t| t.clone()
                        children=|t: String| view! { <ToggleGroupItem value=t.clone() class="rounded-full! px-3">{t}</ToggleGroupItem> }
                    />
                </ToggleGroup>
                <Show when=move || set.tags.with(|t| t.len() > 1)>
                    <Field orientation=Orientation::Horizontal>
                        <Switch checked=set.any />
                        <FieldLabel>"Any of them (off: all of them)"</FieldLabel>
                    </Field>
                </Show>
                <FieldDescription>"None picked: any wallpaper."</FieldDescription>
            </Field>
        </Show>
        <Field>
            <FieldLabel>"Only in folder"</FieldLabel>
            <Input model=set.folder class="font-mono text-xs" attr:placeholder="Any folder" attr:spellcheck="false" />
        </Field>
    }
}

/// Settings → Performance: a live reading of what wallpicker uses, and the
/// switches for what costs the most. The effects apply at once (per machine);
/// the preview size is a setting (Save).
#[component]
fn PerformancePane(full_previews: RwSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let usage = RwSignal::new(None::<api::Usage>);
    // Every two seconds while this section is shown.
    let alive = RwSignal::new(true);
    on_cleanup(move || alive.set(false));
    let poll = move || {
        spawn_local(async move {
            if let Ok(u) = api::app_usage().await {
                usage.try_set(Some(u));
            }
        });
    };
    poll();
    let _ = set_interval_with_handle(
        move || {
            if alive.get_untracked() {
                poll();
            }
        },
        std::time::Duration::from_secs(2),
    )
    .map(|h| on_cleanup(move || h.clear()));
    let part = |label: &'static str, mb: Box<dyn Fn(&api::Usage) -> u32 + Send + Sync>| {
        view! {
            <div class="flex flex-col rounded-xl bg-muted/60 px-3 py-2">
                <span class="text-xs text-muted-foreground">{label}</span>
                <span class="font-extrabold tabular-nums">
                    {move || usage.with(|u| u.as_ref().map_or("…".to_string(), |u| format!("{} MB", mb(u))))}
                </span>
            </div>
        }
    };
    view! {
        <div class="flex flex-col gap-2 rounded-2xl border-2 border-border/60 p-4">
            <div class="flex items-baseline justify-between">
                <span class="text-sm font-bold">"Right now"</span>
                <span class="text-2xl font-black tabular-nums">
                    {move || usage.with(|u| u.as_ref().map_or("…".to_string(), |u| format!("{} MB", u.total_mb)))}
                </span>
            </div>
            <div class="grid grid-cols-4 gap-2">
                {part("App", Box::new(|u| u.app_mb))}
                {part("Interface", Box::new(|u| u.interface_mb))}
                {part("Helpers", Box::new(|u| u.helpers_mb))}
                <div class="flex flex-col rounded-xl bg-muted/60 px-3 py-2">
                    <span class="text-xs text-muted-foreground">"CPU"</span>
                    <span class="font-extrabold tabular-nums">
                        {move || usage.with(|u| u.as_ref().and_then(|u| u.cpu).map_or("…".to_string(), |c| format!("{c:.0}%")))}
                    </span>
                </div>
            </div>
            <p class="text-xs text-muted-foreground">
                "Closing the window to the tray frees the interface; the tray alone is a fraction of this."
            </p>
        </div>
        <Field orientation=Orientation::Horizontal>
            <Switch checked=ctx.glass />
            <FieldLabel>"Frosted glass"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "The blur behind the bars. It's redrawn whenever what's under it moves, which is most of "
            "the CPU while scrolling. Off: solid bars. Applies right away."
        </p>
        <Field orientation=Orientation::Horizontal>
            <Switch checked=ctx.backdrop />
            <FieldLabel>"Wallpaper behind the app"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "A big blurred picture under everything (also in Appearance). Applies right away."
        </p>
        <Field orientation=Orientation::Horizontal>
            <Switch checked=ctx.motion />
            <FieldLabel>"Animations"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "Hops, slides and fades. Off: things change at once. Applies right away."
        </p>
        <Field orientation=Orientation::Horizontal>
            <Switch checked=full_previews />
            <FieldLabel>"Full-size previews in Online details"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "Off: the preview is fitted to 1600 px, plenty for the details view. On: the original, "
            "which for an 8K wallpaper is over 100 MB while it's shown. Save to apply."
        </p>
    }
}
