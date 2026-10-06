//! The app's colors from the current wallpaper: the backend's Material 3
//! palette (Noctalia's generator) mapped onto the theme tokens, as a
//! stylesheet after the built-in one, for light and dark alike. Turned off
//! (Settings → Appearance) or unavailable, the built-in colors stay.

use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::api::{self, Palette};
use crate::ctx::Ctx;

const STYLE_ID: &str = "wallpaper-palette";

/// Theme token → Material 3 color role, and how much of the primary color
/// to mix into it (%). Some schemes (m3-rainbow) give plain grey surfaces;
/// a little of the wallpaper's color makes them feel like its own.
const TOKENS: &[(&str, &str, u8)] = &[
    ("background", "surface", 12),
    ("foreground", "on_surface", 0),
    ("card", "surface_container_low", 14),
    ("card-foreground", "on_surface", 0),
    ("popover", "surface_container", 14),
    ("popover-foreground", "on_surface", 0),
    ("primary", "primary", 0),
    ("primary-foreground", "on_primary", 0),
    ("secondary", "secondary_container", 0),
    ("secondary-foreground", "on_secondary_container", 0),
    ("muted", "surface_container_high", 16),
    ("muted-foreground", "on_surface_variant", 0),
    ("accent", "surface_container_highest", 24),
    ("accent-foreground", "on_surface", 0),
    ("destructive", "error", 0),
    ("border", "outline_variant", 20),
    ("input", "outline_variant", 20),
    ("ring", "primary", 0),
    ("sidebar", "surface_container_low", 14),
    ("sidebar-foreground", "on_surface", 0),
    ("sidebar-primary", "primary", 0),
    ("sidebar-primary-foreground", "on_primary", 0),
    ("sidebar-accent", "surface_container_high", 20),
    ("sidebar-accent-foreground", "on_surface", 0),
    ("sidebar-border", "outline_variant", 20),
    ("sidebar-ring", "primary", 0),
];

fn block(selector: &str, colors: &BTreeMap<String, String>) -> String {
    let primary = colors.get("primary");
    let vars: String = TOKENS
        .iter()
        .filter_map(|(token, role, tint)| {
            let hex = colors.get(*role)?;
            Some(match primary {
                Some(p) if *tint > 0 => {
                    format!("--{token}:color-mix(in oklab, {hex} {}%, {p});", 100 - tint)
                }
                _ => format!("--{token}:{hex};"),
            })
        })
        .collect();
    format!("{selector}{{{vars}}}")
}

/// Over everything else, the color themes' `[data-theme=…].dark` included.
fn css(p: &Palette) -> String {
    format!("{}{}", block(":root:root:root", &p.light), block(":root:root.dark", &p.dark))
}

/// Put `css` in (or take it out, with None).
fn set_style(css: Option<String>) {
    let doc = document();
    let existing = doc.get_element_by_id(STYLE_ID);
    match (css, existing) {
        (Some(css), Some(el)) => el.set_text_content(Some(&css)),
        (Some(css), None) => {
            if let Ok(el) = doc.create_element("style") {
                el.set_id(STYLE_ID);
                el.set_text_content(Some(&css));
                // Last in <head>: it wins over the built-in tokens.
                if let Some(head) = doc.head() {
                    let _ = head.append_child(el.unchecked_ref());
                }
            }
        }
        (None, Some(el)) => el.remove(),
        (None, None) => {}
    }
}

/// The palette's stylesheet as last applied, so a new window starts with it
/// (no switch from the built-in colors a moment after opening).
const CACHE_KEY: &str = "ui.palette_css";

/// Tell the app what the background is now, so its next window starts in
/// that color instead of WebKit's white. After the theme class and the
/// palette have settled.
fn report_background() {
    set_timeout(
        || {
            let Some(body) = document().body() else { return };
            let Ok(Some(style)) = window().get_computed_style(&body) else { return };
            let Ok(color) = style.get_property_value("background-color") else { return };
            // Computed colors come as rgb(), oklab() or color(...): let a
            // 1×1 canvas turn whichever into sRGB bytes.
            let Some(v) = to_rgb(&color) else { return };
            if let [r, g, b] = v[..] {
                spawn_local(async move {
                    let _ = api::window_background(r, g, b).await;
                });
            }
        },
        std::time::Duration::from_millis(100),
    );
}

/// Any CSS color as sRGB bytes, by painting it on a 1×1 canvas.
fn to_rgb(color: &str) -> Option<Vec<u8>> {
    let canvas: web_sys::HtmlCanvasElement = document().create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(1);
    canvas.set_height(1);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    ctx.set_fill_style_str(color);
    ctx.fill_rect(0.0, 0.0, 1.0, 1.0);
    let data = ctx.get_image_data(0.0, 0.0, 1.0, 1.0).ok()?.data();
    Some(data.0[..3].to_vec())
}

/// Follow the wallpaper: again whenever a monitor's wallpaper or the setting
/// changes.
pub(crate) fn install(ctx: Ctx) {
    // The last palette at once, before the settings are even read.
    if let Some(cached) = crate::prefs::get(CACHE_KEY).filter(|c| !c.is_empty()) {
        set_style(Some(cached));
    }
    let seq = StoredValue::new(0u64);
    Effect::new(move |_| {
        ctx.state.track();
        // Not known yet: keep what's showing (the cached palette, if any).
        let Some(on) = ctx.settings.with(|s| s.as_ref().map(|s| s.theme_from_wallpaper)) else { return };
        let n = seq.get_value() + 1;
        seq.set_value(n);
        if !on {
            set_style(None);
            crate::prefs::set(CACHE_KEY, "");
            report_background();
            return;
        }
        spawn_local(async move {
            let palette = api::wallpaper_palette().await.ok().flatten();
            if seq.get_value() == n {
                let style = palette.as_ref().map(css);
                crate::prefs::set(CACHE_KEY, style.as_deref().unwrap_or(""));
                set_style(style);
                report_background();
            }
        });
    });
    // Light / dark and the color theme change the background too.
    let theme = nocomponents::utils::theme::use_theme();
    Effect::new(move |_| {
        theme.resolved.track();
        theme.color.track();
        report_background();
    });
}

/// The current wallpaper behind the app: the colors' monitor's, else the
/// first one's (its cached thumbnail, blurred and dimmed), under a wash of
/// the background color.
#[component]
pub(crate) fn Backdrop() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    Effect::new(move |_| crate::prefs::set("ui.backdrop", if ctx.backdrop.get() { "1" } else { "0" }));
    let src = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let chosen = ctx.settings.with(|s| s.as_ref().and_then(|s| s.theme_monitor.clone()));
        let first = ctx.monitors.with(|ms| ms.first().map(|m| m.name.clone()));
        let path = ctx.state.with(|s| {
            chosen
                .and_then(|m| s.current.get(&m).cloned())
                .or_else(|| first.and_then(|m| s.current.get(&m).cloned()))
                .or_else(|| s.current.values().next().cloned())
        });
        let Some(path) = path else {
            src.set(None);
            return;
        };
        spawn_local(async move {
            let url = api::thumbnail(&path.to_string_lossy()).await.ok().map(|t| api::asset_url(&t));
            src.try_set(url);
        });
    });
    view! {
        <Show when=move || ctx.backdrop.get() && src.with(Option::is_some)>
            <div class="pointer-events-none fixed inset-0 z-0 overflow-hidden" aria-hidden="true">
                <img
                    src=move || src.get().unwrap_or_default()
                    alt=""
                    class="size-full scale-110 object-cover opacity-70 blur-2xl saturate-150"
                />
                <div class="absolute inset-0 bg-gradient-to-b from-background/30 via-background/55 to-background/80" />
            </div>
        </Show>
    }
}
