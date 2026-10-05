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

/// Theme token → Material 3 color role.
const TOKENS: &[(&str, &str)] = &[
    ("background", "surface"),
    ("foreground", "on_surface"),
    ("card", "surface_container_low"),
    ("card-foreground", "on_surface"),
    ("popover", "surface_container"),
    ("popover-foreground", "on_surface"),
    ("primary", "primary"),
    ("primary-foreground", "on_primary"),
    ("secondary", "secondary_container"),
    ("secondary-foreground", "on_secondary_container"),
    ("muted", "surface_container_high"),
    ("muted-foreground", "on_surface_variant"),
    ("accent", "surface_container_highest"),
    ("accent-foreground", "on_surface"),
    ("destructive", "error"),
    ("border", "outline_variant"),
    ("input", "outline_variant"),
    ("ring", "primary"),
    ("sidebar", "surface_container_low"),
    ("sidebar-foreground", "on_surface"),
    ("sidebar-primary", "primary"),
    ("sidebar-primary-foreground", "on_primary"),
    ("sidebar-accent", "surface_container_high"),
    ("sidebar-accent-foreground", "on_surface"),
    ("sidebar-border", "outline_variant"),
    ("sidebar-ring", "primary"),
];

fn block(selector: &str, colors: &BTreeMap<String, String>) -> String {
    let vars: String = TOKENS
        .iter()
        .filter_map(|(token, role)| colors.get(*role).map(|hex| format!("--{token}:{hex};")))
        .collect();
    format!("{selector}{{{vars}}}")
}

fn css(p: &Palette) -> String {
    format!("{}{}", block(":root", &p.light), block(".dark", &p.dark))
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

/// Follow the wallpaper: again whenever a monitor's wallpaper or the setting
/// changes.
pub(crate) fn install(ctx: Ctx) {
    let seq = StoredValue::new(0u64);
    Effect::new(move |_| {
        ctx.state.track();
        let on = ctx.settings.with(|s| s.as_ref().is_some_and(|s| s.theme_from_wallpaper));
        let n = seq.get_value() + 1;
        seq.set_value(n);
        if !on {
            set_style(None);
            return;
        }
        spawn_local(async move {
            let palette = api::wallpaper_palette().await.ok().flatten();
            if seq.get_value() == n {
                set_style(palette.as_ref().map(css));
            }
        });
    });
}
