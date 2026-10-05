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

const FIT_MODES: &[(&str, &str)] = &settings::FIT_MODES;

const SECTION_GENERAL: &str = "general";
const SECTION_APPEARANCE: &str = "appearance";
const SECTION_DOWNLOADS: &str = "downloads";
const SECTION_SLIDESHOW: &str = "slideshow";
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

const THEMES: &[(&str, &str)] = &[("system", "System"), ("light", "Light"), ("dark", "Dark")];

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
    let api_key = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let fit = RwSignal::new(String::new());
    // Monitor name -> its own fit ("cover", ...); absent: the general one.
    let monitor_fits = RwSignal::new(std::collections::BTreeMap::<String, String>::new());
    let konachan_site = RwSignal::new(KonachanSite::default().as_str().to_string());
    let backend = RwSignal::new(WallpaperBackend::default().as_str().to_string());
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
            api_key.set(s.wallhaven_api_key.clone().unwrap_or_default());
            username.set(s.wallhaven_username.clone().unwrap_or_default());
            konachan_site.set(s.konachan_site.as_str().to_string());
            backend.set(s.wallpaper_backend.as_str().to_string());
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
            _ => WallpaperBackend::Hyprpaper,
        };
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
                                <Show when=move || backend.get() == WallpaperBackend::Noctalia.as_str()>
                                    <p class="text-sm text-muted-foreground">
                                        "Noctalia keeps the wallpapers itself, so they come back after a restart. "
                                        "Its fill mode and transitions are set in Noctalia's settings (Wallpaper). "
                                        "Turn Noctalia's wallpaper on there, and stop hyprpaper (take it out of "
                                        "your Hyprland autostart): both draw the background."
                                    </p>
                                </Show>
                                <div class="grid gap-6 sm:grid-cols-2">
                                    <Show when=move || backend.get() == WallpaperBackend::Hyprpaper.as_str()>
                                        <Field>
                                            <FieldLabel>"Fit mode"</FieldLabel>
                                            <Choice items=FIT_MODES.to_vec() model=fit />
                                            <FieldDescription>
                                                "For every monitor, unless it has its own below; a wallpaper's own fit (right-click → Fit) wins over both. Needs a newer hyprpaper: leave it on default if applying fails."
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
                                    <FieldLabel>"Theme"</FieldLabel>
                                    <Choice items=THEMES.to_vec() model=theme_choice />
                                    <FieldDescription>"Applies right away."</FieldDescription>
                                </Field>
                                <Field orientation=Orientation::Horizontal>
                                    <Switch checked=theme_from_wallpaper />
                                    <FieldLabel>"Colors from the wallpaper"</FieldLabel>
                                </Field>
                                // Plain text: a FieldDescription needs to sit inside a Field.
                                <p class="-mt-2 text-sm text-muted-foreground">
                                    "The app takes its colors from the wallpaper on your first monitor, the "
                                    "way Noctalia makes them (needs Noctalia installed). Save to apply."
                                </p>
                                <NoctaliaColorsField />
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
    favorites: RwSignal<bool>,
    tags: RwSignal<Vec<String>>,
    any: RwSignal<bool>,
    folder: RwSignal<String>,
    monitors: RwSignal<Vec<String>>,
    each: RwSignal<bool>,
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
            favorites: RwSignal::new(false),
            tags: RwSignal::new(Vec::new()),
            any: RwSignal::new(false),
            folder: RwSignal::new(String::new()),
            monitors: RwSignal::new(Vec::new()),
            each: RwSignal::new(true),
        }
    }

    fn load(self, s: &settings::Slideshow) {
        self.enabled.set(s.enabled);
        self.minutes.set(s.minutes.to_string());
        self.favorites.set(s.favorites);
        self.tags.set(s.tags.clone());
        self.any.set(s.any);
        self.folder.set(s.folder.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
        self.monitors.set(s.monitors.clone());
        self.each.set(s.each);
    }

    fn read(self, mut s: settings::Slideshow) -> Result<settings::Slideshow, &'static str> {
        s.enabled = self.enabled.get_untracked();
        s.minutes = self.minutes.get_untracked().parse().unwrap_or(30).clamp(1, 24 * 60);
        s.favorites = self.favorites.get_untracked();
        s.tags = self.tags.get_untracked();
        s.any = self.any.get_untracked();
        let folder = self.folder.get_untracked().trim().to_string();
        s.folder = if folder.is_empty() {
            None
        } else {
            let p = PathBuf::from(&folder);
            // `has_root`: `is_absolute` is always false on wasm.
            if !p.has_root() && !p.starts_with("~") {
                return Err("The slideshow folder must be an absolute path (or start with ~).");
            }
            Some(p)
        };
        s.monitors = self.monitors.get_untracked();
        s.each = self.each.get_untracked();
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
                Ok(s) if s.enabled => s.next_in.map(|n| format!("Next change in about {} min.", n.div_ceil(60).max(1))),
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
        <Field orientation=Orientation::Horizontal>
            <Switch checked=form.favorites />
            <FieldLabel>"Only favorites"</FieldLabel>
        </Field>
        <Show when=move || tags.with(|t| !t.is_empty())>
            <Field>
                <FieldLabel>"Only with these tags"</FieldLabel>
                <ToggleGroup
                    kind=ToggleGroupType::Multiple
                    variant=ToggleVariant::Outline
                    size=ToggleSize::Sm
                    value=form.tags
                    class="flex-wrap gap-1.5"
                >
                    <For
                        each=move || tags.get()
                        key=|t| t.clone()
                        children=|t: String| view! { <ToggleGroupItem value=t.clone() class="rounded-full! px-3">{t}</ToggleGroupItem> }
                    />
                </ToggleGroup>
                <Show when=move || form.tags.with(|t| t.len() > 1)>
                    <Field orientation=Orientation::Horizontal>
                        <Switch checked=form.any />
                        <FieldLabel>"Any of them (off: all of them)"</FieldLabel>
                    </Field>
                </Show>
                <FieldDescription>"None picked: any wallpaper."</FieldDescription>
            </Field>
        </Show>
        <Field>
            <FieldLabel>"Only in folder"</FieldLabel>
            <Input model=form.folder class="font-mono text-xs" attr:placeholder="Any folder" attr:spellcheck="false" />
        </Field>
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
